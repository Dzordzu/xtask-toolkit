use std::collections::HashMap;
use std::path::PathBuf;

use crate::cargo::CargoToml;
use crate::git::{LastCommitError, OriginUrl, last_commit_date};
use crate::linux_utils::SystemdUnit;
use crate::package_utils::buildhost;
use rpm::PackageBuilder;

#[derive(Debug)]
pub struct Package {
    cargo_toml: CargoToml,
    create_user: Option<String>,
    systemd_units: HashMap<PathBuf, SystemdUnit>,
    arch: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum PackageError {
    #[error(transparent)]
    CommitDateError(#[from] LastCommitError),

    #[error(transparent)]
    GitOriginError(xshell::Error),

    #[error("Could not transform git repository to http url")]
    GitTransformError,

    #[error("Missing key {0} in Cargo.toml")]
    MissingKey(String),

    #[error(transparent)]
    SystemdFileError(rpm::Error),
}

impl Package {
    pub fn new(cargo_toml: CargoToml) -> Self {
        Self {
            cargo_toml,
            create_user: None,
            systemd_units: HashMap::new(),
            arch: None,
        }
    }

    pub fn with_arch(mut self, arch: String) -> Self {
        self.arch = Some(arch);
        self
    }

    pub fn with_user(mut self, user: String) -> Self {
        self.create_user = Some(user);
        self
    }

    pub fn with_systemd_unit(mut self, path: PathBuf) -> Result<Self, Self> {
        // this little monster is caused because of the false borrow checker error (self moved)
        let unit_name = path.file_name();
        if path.file_name().is_none() {
            return Err(self);
        }
        let unit_name = unit_name.unwrap().to_string_lossy().to_string();

        self.systemd_units
            .insert(path.clone(), SystemdUnit(unit_name));

        Ok(self)
    }

    pub fn with_sytemd_units(mut self, paths: Vec<PathBuf>) -> Result<Self, Self> {
        let unit_names= paths.iter().filter(|x| x.file_name().is_some()).count();
        if unit_names != paths.len() {
            return Err(self);
        } else {
            for path in paths {
                self = self.with_systemd_unit(path).unwrap(); // this should be checked by unit_names
            }
            Ok(self)
        }
    }

    fn rpm_post_uninstall() -> String {
        crate::linux_utils::SystemdUnit::bash_reload_daemon()
    }

    fn rpm_post_install() -> String {
        crate::linux_utils::SystemdUnit::bash_reload_daemon()
    }

    fn rpm_pre_install(&self) -> String {
        if let Some(create_user) = &self.create_user {
            crate::linux_utils::LinuxUser(create_user.clone()).bash_add()
        } else {
            "".to_string()
        }
    }

    fn rpm_pre_uninstall(&self) -> String {
        let uninstallation_units = self.systemd_units.iter().fold(Vec::new(), |mut acc, unit| {
            if unit.0.ends_with(".timer") || unit.0.ends_with(".service") {
                acc.push(unit);
            }
            acc
        });

        let remove = uninstallation_units.iter().fold(String::new(), |acc, x| {
            format!("{}\n{}", acc, x.1.bash_disable_and_stop())
        });

        let restart = uninstallation_units.iter().fold(String::new(), |acc, x| {
            format!("{}\n{}", acc, x.1.bash_restart_if_active())
        });

        format!(
            r#"
        IS_UPGRADED="$1"
        case "$IS_UPGRADED" in
           0) # This is a yum remove.
              {remove}
           ;;
           1) # This is a yum upgrade.
              {restart}
              exit 0;
           ;;
         esac
    "#
        )
    }

    fn add_hooks(&self, builder: rpm::PackageBuilder) -> Result<PackageBuilder, PackageError> {
        let builder = builder
            .pre_install_script(self.rpm_pre_install())
            .post_install_script(Package::rpm_post_install())
            .post_uninstall_script(Package::rpm_post_uninstall())
            .pre_uninstall_script(self.rpm_pre_uninstall());

        self.systemd_units
            .iter()
            .fold(Ok(builder), |builder, unit| {
                if let Ok(builder) = builder {
                    let dest_path = format!("/etc/systemd/system/{}", unit.0.to_string_lossy());
                    let file_opts =
                        rpm::FileOptions::new(dest_path).mode(rpm::FileMode::regular(0o644));
                    builder
                        .with_file(unit.0, file_opts)
                        .map_err(PackageError::SystemdFileError)
                } else {
                    builder
                }
            })
    }

    pub fn builder(&self) -> Result<PackageBuilder, PackageError> {
        let last_commit_date =
            last_commit_date().map_err(|error| PackageError::CommitDateError(error))?;

        let buildhost = buildhost();
        let compression = rpm::CompressionType::Gzip;

        let url = OriginUrl::new()
            .map_err(|error| PackageError::GitOriginError(error))?
            .to_http()
            .map_err(|_| PackageError::GitTransformError)?;

        let package_name = self
            .cargo_toml
            .name()
            .ok_or(PackageError::MissingKey("name".to_string()))?;

        let version = self
            .cargo_toml
            .version()
            .ok_or(PackageError::MissingKey("version".to_string()))?;

        let license = self.cargo_toml.license().unwrap_or("MIT".to_string());

        let arch = &match self.arch {
            Some(ref arch) => &arch,
            None => std::env::consts::ARCH,
        };

        let summary = self.cargo_toml.description().unwrap_or_default();

        let vendor = self
            .cargo_toml
            .authors()
            .unwrap_or(vec!["".to_string()])
            .first()
            .unwrap_or(&"".to_string())
            .to_string();

        let result = rpm::PackageBuilder::new(&package_name, &version, &license, arch, &summary)
            .source_date(last_commit_date)
            .vendor(&vendor)
            .build_host(&buildhost)
            .compression(compression)
            .url(url.get());

        self.add_hooks(result)
    }
}
