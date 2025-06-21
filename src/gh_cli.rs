use std::path::PathBuf;

use xshell::{Shell, cmd};

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct GhResponse {
    pub tag_name: String,
    pub is_draft: bool,
    pub is_prelease: bool,
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
            prelease: value.is_prelease,
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
    pub fn new(name: &str, version: &str, mode: ReleaseMode) -> Option<Self> {
        let version = semver::Version::parse(version).ok()?;
        Some(Release {
            name: name.to_string(),
            version,
            draft: matches!(mode, ReleaseMode::Draft),
            prelease: matches!(mode, ReleaseMode::Prerelease),
        })
    }

    pub fn get_from_gh() -> Result<Vec<Self>, GetFromGHError> {
        let sh = Shell::new()?;
        let previous_releases: Vec<GhResponse> =
            serde_json::from_str(&cmd!(sh, "gh release list --json tagName").read()?)?;

        Ok(previous_releases
            .into_iter()
            .filter_map(|x| Self::try_from(x).ok())
            .collect())
    }

    pub fn release(&self, files: Vec<PathBuf>) -> Result<(), xshell::Error> {
        let sh = Shell::new()?;
        let files = files
            .iter()
            .map(|x| x.display().to_string())
            .collect::<Vec<_>>();

        let name = self.name.clone();

        let draft = if self.draft { " --draft" } else { "" };

        sh.cmd("gh")
            .args(["release", "create", "generate-notes", draft, &name])
            .args(files)
            .run()
    }
}
