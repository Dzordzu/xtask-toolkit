use std::fs::read_dir;
use std::path::{Path, PathBuf};
use std::{env, fs};

use xshell::{Shell, cmd};

pub enum ProjectRootError {
    Io(std::io::Error),
    MissingCargoLock,
}

impl From<std::io::Error> for ProjectRootError {
    fn from(e: std::io::Error) -> Self {
        ProjectRootError::Io(e)
    }
}

pub fn get_project_root() -> Result<PathBuf, ProjectRootError> {
    let path = env::current_dir()?;
    let path_ancestors = path.as_path().ancestors();

    for p in path_ancestors {
        let has_cargo = read_dir(p)?.any(|p| p.unwrap().file_name() == *"Cargo.lock");
        if has_cargo {
            return Ok(PathBuf::from(p));
        }
    }
    Err(ProjectRootError::MissingCargoLock)
}

#[derive(Debug)]
pub struct CargoToml(PathBuf);

impl CargoToml {
    pub fn find_all<P: AsRef<Path>>(dir: P) -> Vec<Self> {
        let mut matches = Vec::new();
        let target_name = "Cargo.toml";

        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();

                if path.is_dir() {
                    matches.extend(Self::find_all(&path));
                } else if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                    if file_name == target_name {
                        matches.push(Self(path));
                    }
                }
            }
        }

        matches
    }
    pub fn find_first<P: AsRef<Path>>(dir: P) -> Option<Self> {
        let target_name = "Cargo.toml";

        let mut entries = match fs::read_dir(&dir) {
            Ok(e) => e.filter_map(Result::ok).collect::<Vec<_>>(),
            Err(_) => return None,
        };

        entries.sort_by_key(|e| e.path());

        for entry in &entries {
            let path = entry.path();

            if path.is_file() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name == target_name {
                        return Some(Self(path));
                    }
                }
            }
        }

        for entry in entries {
            let path = entry.path();
            if path.is_dir() {
                if let Some(found) = Self::find_first(&path) {
                    return Some(found);
                }
            }
        }

        None
    }

    fn get_toml_key<'a, T>(&self, keypath: &[&str]) -> Option<T>
    where
        T: serde::Deserialize<'a>,
    {
        let mut result = toml::from_str::<toml::Value>(&fs::read_to_string(&self.0).ok()?).ok()?;

        for key in keypath {
            if result.is_table() && result.get(key).is_some() {
                let table = result.as_table_mut().unwrap();
                result = table.remove(*key).unwrap();
            } else {
                break;
            }
        }

        result.try_into().ok()
    }

    pub fn version(&self) -> Option<String> {
        self.get_toml_key(&["package", "version"])
    }

    pub fn name(&self) -> Option<String> {
        self.get_toml_key(&["package", "name"])
    }

    pub fn license(&self) -> Option<String> {
        self.get_toml_key(&["package", "license"])
    }

    pub fn authors(&self) -> Option<Vec<String>> {
        self.get_toml_key(&["package", "authors"])
    }

    pub fn description(&self) -> Option<String> {
        self.get_toml_key(&["package", "description"])
    }
}

pub struct BinaryBuild {
    pub projects: Vec<String>,
    pub target: Option<String>,
}

impl BinaryBuild {
    pub fn new(projects: &[&str], target: Option<&str>) -> Self {
        Self {
            projects: projects.iter().map(|x| x.to_string()).collect(),
            target: target.map(|x| x.to_string()),
        }
    }

    pub fn build(&self) -> Result<(), xshell::Error> {
        let sh = Shell::new()?;

        let projects: Vec<String> = self.projects.iter().map(|x| format!("-p={}", x)).collect();

        let cmd = sh.cmd("cargo").args([
            "build",
            "--release",
            "--target",
            "x86_64-unknown-linux-musl",
        ]);

        let cmd = if let Some(target) = &self.target {
            cmd.args(["--target", target])
        } else {
            cmd
        };

        cmd.args(projects).read()?;

        Ok(())
    }
}

pub fn force_fmt() -> Result<(), xshell::Error> {
    let sh = Shell::new()?;
    cmd!(sh, "cargo fmt").read()?;
    cmd!(sh, "cargo clippy --fix --allow-dirty --allow-staged").read()?;
    Ok(())
}
