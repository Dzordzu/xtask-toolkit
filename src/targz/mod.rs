use std::path::{Path, PathBuf};

pub struct DirCompress {
    dir: PathBuf,
    filter_extensions: Vec<String>,
    filter_filename_regex: Option<regex::Regex>,
    filter_filenames: Vec<String>,
    search_subdirs: bool,
}

impl DirCompress {
    pub fn new<T>(dir: T) -> Option<Self> where T: Into<PathBuf> {
        let dir : PathBuf = dir.into();
        dir.is_dir().then_some(Self {
            dir,
            filter_filenames: Vec::new(),
            filter_extensions: Vec::new(),
            filter_filename_regex: None,
            search_subdirs: false,
        })
    }

    pub fn search_subdirs(&mut self) -> &mut Self {
        self.search_subdirs = true;
        self
    }

    pub fn filter_extension(&mut self, extension: &str) -> &mut Self {
        self.filter_extensions.push(extension.to_string());
        self
    }

    pub fn filter_filename_regex(&mut self, regex: regex::Regex) -> &mut Self {
        self.filter_filename_regex = Some(regex);
        self
    }

    pub fn filter_filename(&mut self, filename: &str) -> &mut Self {
        self.filter_filenames.push(filename.to_string());
        self
    }

    pub fn compress(&self, output_file: &Path) -> Result<(), std::io::Error> {
        let walkdir = walkdir::WalkDir::new(&self.dir);
        let walkdir = if self.search_subdirs {
            walkdir.max_depth(1)
        } else {
            walkdir
        };

        let files = walkdir
            .into_iter()
            .filter_map(|x| {
                x.ok()
                    .and_then(|x| {
                        let filename = x.file_name().to_string_lossy();

                        for name in &self.filter_filenames {
                            if filename == name.as_str() {
                                return Some(x);
                            }
                        }

                        for ext in &self.filter_extensions {
                            if x.file_name().to_string_lossy().ends_with(ext) {
                                return Some(x);
                            }
                        }

                        if let Some(regex) = &self.filter_filename_regex {
                            if regex.is_match(filename.as_ref()) {
                                return Some(x);
                            }
                        }

                        None
                    })
                    .and_then(|x| x.path().is_file().then_some(x))
            })
            .map(|x| x.path().to_path_buf())
            .collect::<Vec<PathBuf>>();

        let dest_file = std::fs::File::create(&output_file)?;
        let enc = flate2::write::GzEncoder::new(&dest_file, flate2::Compression::default());
        let mut builder = tar::Builder::new(enc);

        for src in files {
            builder.append_path_with_name(
                &src,
                src.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
            )?;
        }

        Ok(())
    }
}
