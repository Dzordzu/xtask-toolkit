use std::path::{Path, PathBuf};

pub struct LinuxUser(pub String);
pub struct LinuxGroup(pub String);

impl LinuxUser {
    pub fn bash_add(&self) -> String {
        format!(
            r#"
        if [ -z "$(getent passwd | grep {account})" ]; then 
            useradd -r {account};
        fi
    "#,
            account = &self.0
        )
    }

    pub fn bash_remove(&self) -> String {
        format!("userdel -r {};", &self.0)
    }
}

impl LinuxGroup {
    pub fn bash_add(&self) -> String {
        format!(
            r#"
        if [ -z "$(getent group | grep {group})" ]; then 
            groupadd -r {group};
        fi
    "#,
            group = &self.0
        )
    }

    pub fn bash_remove(&self) -> String {
        format!("groupdel {};", &self.0)
    }
}

#[derive(Debug)]
pub struct SystemdUnit(pub String);

impl TryFrom<&Path> for SystemdUnit
{
    type Error = ();

    fn try_from(value: &Path) -> Result<Self, Self::Error> {
        Ok(Self(
            value
                .file_name()
                .ok_or(())?
                .to_string_lossy()
                .to_string(),
        ))
    }
}

impl TryFrom<&PathBuf> for SystemdUnit
{
    type Error = ();

    fn try_from(value: &PathBuf) -> Result<Self, Self::Error> {
        Self::try_from(value.as_path())
    }
}

impl SystemdUnit {
    pub fn bash_reload_daemon() -> String {
        format!("systemctl daemon-reload;")
    }

    pub fn bash_disable_and_stop(&self) -> String {
        let unit = &self.0;
        format!(
            r#"
            if [ $(systemctl list-unit-files {unit} &> /dev/null; echo $?) -eq 0 ]; then
                systemctl disable {unit};
                systemctl stop {unit};
            fi;
        "#
        )
    }

    pub fn bash_restart_if_active(&self) -> String {
        format!(
            "systemctl is-active --quiet {} && systemctl restart {};",
            &self.0, &self.0
        )
    }
}
