use std::path::Path;

use xshell::{cmd, Shell};

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct GhResponse {
    pub tag_name: String,

    #[serde(default)]
    pub is_draft: bool,

    #[serde(default)]
    pub is_prerelease: bool,
}

pub struct Release {
    pub name: String,
    pub version: semver::Version,
    pub draft: bool,
    pub prelease: bool,
}

impl TryFrom<GhResponse> for Release {
    type Error = ();
    fn try_from(value: GhResponse) -> Result<Self, Self::Error> {
        let (name, version) = value.tag_name.rsplit_once('-').ok_or(())?;

        let version = semver::Version::parse(version).map_err(|_| ())?;

        Ok(Release {
            name: name.to_string(),
            version,
            draft: value.is_draft,
            prelease: value.is_prerelease,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GetFromGHError {
    #[error(transparent)]
    XShellError(#[from] xshell::Error),

    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),
}

#[derive(Default)]
pub enum ReleaseMode {
    #[default]
    Normal,

    Draft,
    Prerelease,
}

impl Release {
    pub fn new(name: &str, version: &str) -> Result<Self, semver::Error> {
        let version = semver::Version::parse(version)?;
        Ok(Release {
            name: name.to_string(),
            version,
            draft: false,
            prelease: false,
        })
    }

    pub fn with_release_mode(&mut self, mode: ReleaseMode) -> &mut Self {
        self.draft = matches!(mode, ReleaseMode::Draft);
        self.prelease = matches!(mode, ReleaseMode::Prerelease);
        self
    }

    pub fn get_from_gh() -> Result<Vec<Self>, GetFromGHError> {
        let sh = Shell::new()?;
        let previous_releases: Vec<GhResponse> = serde_json::from_str(
            &cmd!(sh, "gh release list --json tagName,isDraft,isPrerelease").read()?,
        )?;

        Ok(previous_releases
            .into_iter()
            .filter_map(|x| Self::try_from(x).ok())
            .collect())
    }

    pub fn release<T, I>(&self, files: T) -> Result<String, xshell::Error>
    where
        T: IntoIterator<Item = I>,
        I: AsRef<Path> + Sized,
    {
        let sh = Shell::new()?;
        let files = files
            .into_iter()
            .map(|x| x.as_ref().display().to_string())
            .collect::<Vec<_>>();

        let name = self.name.clone();

        let cmd = sh.cmd("gh")
            .args(["release", "create", &name, "--generate-notes"]);

        let cmd = if self.draft { cmd.arg(" --draft") } else { cmd };

        cmd
            .args(files)
            .read()
    }
}
