use std::collections::HashMap;
use std::path::Path;

pub use sha2;
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct Checksum(String);

pub const UNKNOWN_FILENAME: &str = "unknown";

fn strfilename(path: &std::path::Path) -> String {
    path.file_name()
        .map(|x| x.to_string_lossy().to_string())
        .unwrap_or(UNKNOWN_FILENAME.to_string())
}

pub trait PathChecksum {
    fn calculate_sha256(&self) -> Result<Checksum, std::io::Error>;
    fn calculate_sha256_filtered(
        &self,
        filter: fn(&std::path::Path) -> bool,
    ) -> Result<Checksum, std::io::Error>;
    fn calculate_entries_sha256(&self) -> Result<HashMap<String, Checksum>, std::io::Error>;
}

impl PathChecksum for Path {
    fn calculate_sha256_filtered(
        &self,
        filter: fn(&std::path::Path) -> bool,
    ) -> Result<Checksum, std::io::Error> {
        if self.is_file() {
            let binary_content = std::fs::read(&self)?;

            let mut hasher = Sha256::new();
            hasher.update(&binary_content);
            Ok(Checksum(format!("{:x}", hasher.finalize())))
        } else {
            let mut result = String::new();

            for entry in WalkDir::new(self)
                .sort_by_file_name()
                .into_iter()
                .filter(|entry| entry.as_ref().is_ok_and(|x| filter(x.path())))
                .filter_map(Result::ok)
            {
                if entry.file_type().is_file() {
                    result += &entry.path().calculate_sha256()?.0;
                }
            }

            let mut hasher = Sha256::new();
            hasher.update(&result);
            Ok(Checksum(format!("{:x}", hasher.finalize())))
        }
    }

    fn calculate_entries_sha256(&self) -> Result<HashMap<String, Checksum>, std::io::Error> {
        if !self.is_dir() {
            let checksum = self.calculate_sha256()?;
            return Ok(HashMap::from([(strfilename(self), checksum)]));
        }

        let mut result = HashMap::new();
        for entry in self.read_dir()? {
            let entry_path = entry?.path();
            let checksum = entry_path.calculate_sha256()?;
            let filename = strfilename(&entry_path);

            result.insert(filename, checksum);
        }

        Ok(result)
    }

    fn calculate_sha256(&self) -> Result<Checksum, std::io::Error> {
        self.calculate_sha256_filtered(|_| true)
    }
}

impl Checksum {
    pub fn get(&self) -> &str {
        self.0.as_str()
    }

    pub fn string(&self) -> String {
        self.0.clone()
    }
}

pub trait ChecksumsToFile {
    fn save_checksum(&self, path: &std::path::Path) -> Result<(), std::io::Error>;
}

impl<T> ChecksumsToFile for T
where
    T: Iterator<Item = (String, Checksum)> + Clone
{
    fn save_checksum(&self, path: &std::path::Path) -> Result<(), std::io::Error> {
        use std::io::Write;

        let mut file = std::fs::File::create(path)?;

        let checksum_contents = self.clone().fold(String::new(), |acc, x| {
            format!("{}{} {}\n", acc, x.0, x.1.get())
        });

        file.write(checksum_contents.as_bytes())?;
        Ok(())
    }
}
