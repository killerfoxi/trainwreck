use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum GtfsError {
    #[error("failed to open archive at {path}")]
    Open {
        path: PathBuf,
        #[source]
        source: zip::result::ZipError,
    },

    #[error("missing required file \"{name}\" in archive")]
    MissingFile { name: &'static str },

    #[error("failed to read entry \"{name}\" from archive")]
    ReadEntry {
        name: &'static str,
        #[source]
        source: zip::result::ZipError,
    },

    #[error("failed to parse CSV in \"{file}\"")]
    Csv {
        file: &'static str,
        #[source]
        source: csv::Error,
    },
}
