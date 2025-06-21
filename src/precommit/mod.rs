use minijinja::{Environment, context};

pub const PRECOMMIT_TEMPLATE: &str = include_str!("precommit.py.j2");

#[derive(Copy, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Features {
    pub cargo: bool,
    pub taplo: bool,
    pub gitleaks: bool,
}

impl Default for Features {
    fn default() -> Self {
        Self {
            cargo: true,
            taplo: true,
            gitleaks: true,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PrecommitError {
    #[error(transparent)]
    JinjaError(#[from] minijinja::Error),

    #[error(transparent)]
    WriteError(#[from] std::io::Error),

    #[error("Project root cannot be found")]
    ProjectRootError,
}

impl From<xshell::Error> for PrecommitError {
    fn from(_: xshell::Error) -> Self {
        PrecommitError::ProjectRootError
    }
}

pub fn install_precommit(features: Features) -> Result<(), PrecommitError> {
    let mut env = Environment::new();
    env.add_template("precommit", PRECOMMIT_TEMPLATE)?;
    let tmpl = env.get_template("precommit")?;
    let context = context!(
        features => features,
    );
    let rendered = tmpl.render(context)?;

    let root = crate::git::get_root_path()?;
    let dest = root.join(".git").join("hooks").join("pre-commit");

    std::fs::write(dest, rendered.as_bytes())?;

    Ok(())
}
