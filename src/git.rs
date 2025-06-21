use std::path::PathBuf;

use chrono::{DateTime, Utc};
use xshell::{Shell, cmd};

pub fn create_and_push_tag(tag: &str) -> Result<(), xshell::Error> {
    let sh = Shell::new()?;

    cmd!(sh, "git tag {tag}").run()?;
    cmd!(sh, "git push origin {tag}").run()?;
    Ok(())
}

pub fn has_tag(tag: &str) -> Result<bool, xshell::Error> {
    let sh = Shell::new()?;

    let tags = cmd!(sh, "git tag")
        .read()?
        .split('\n')
        .map(|x| x.trim().to_string())
        .collect::<Vec<_>>();

    Ok(tags.iter().any(|x| x == tag))
}

pub fn unstaged_changes() -> Result<bool, xshell::Error> {
    let sh = Shell::new()?;
    Ok(!cmd!(sh, "git status --porcelain").read()?.is_empty())
}

#[derive(Debug, thiserror::Error)]
pub enum LastCommitError {
    #[error(transparent)]
    XShellError(xshell::Error),
    #[error("Could not parse timestamp value. Not an integer")]
    ParseIntError,
    #[error("Not a timestamp")]
    NotATimestamp,
}

impl From<xshell::Error> for LastCommitError {
    fn from(value: xshell::Error) -> Self {
        LastCommitError::XShellError(value)
    }
}

pub fn last_commit_date() -> Result<DateTime<Utc>, LastCommitError> {
    let sh = Shell::new()?;
    DateTime::from_timestamp(
        cmd!(sh, "git show --no-patch --format=%ct HEAD")
            .read()?
            .parse()
            .map_err(|_| LastCommitError::ParseIntError)?,
        0,
    )
    .ok_or_else(|| LastCommitError::NotATimestamp)
}

pub fn get_root_path() -> Result<PathBuf, xshell::Error> {
    let sh = Shell::new()?;
    Ok(PathBuf::from(cmd!(sh, "git rev-parse --show-toplevel").read()?))
}

pub struct OriginUrl(pub String);

impl OriginUrl {
    pub fn new() -> Result<OriginUrl, xshell::Error> {
        let sh = Shell::new()?;
        Ok(OriginUrl(cmd!(sh, "git remote get-url origin").read()?))
    }

    pub fn to_http(mut self) -> Result<Self, ()> {
        if self.0.starts_with("https://") || self.0.starts_with("http://") {
            Ok(self)
        } else if self.0.starts_with("git@") && self.0.contains("github") {
            self.0 = self.0.replace("git@", "https://").replacen(":", "/", 1);
            Ok(self)
        } else {
            Err(())
        }
    }

    pub fn get(&self) -> &str {
        &self.0
    }
}
