use std::{io, path::PathBuf};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum InstanceError {
    #[error("instance name cannot be empty")]
    EmptyName,

    #[error("Minecraft version cannot be empty")]
    EmptyMinecraftVersion,

    #[error("invalid instance id: {0}")]
    InvalidId(String),

    #[error("instance already exists: {0}")]
    AlreadyExists(String),

    #[error("instance not found: {0}")]
    NotFound(String),

    #[error("unsupported instance schema version {found}; supported version is {supported}")]
    UnsupportedSchemaVersion { found: u32, supported: u32 },

    #[error("instance storage id mismatch: directory is {directory_id}, file contains {file_id}")]
    StorageIdMismatch {
        directory_id: String,
        file_id: String,
    },

    #[error("refusing to use symbolic link as instance storage: {0}")]
    SymlinkStorage(PathBuf),

    #[error("invalid instance storage entry: {0}")]
    InvalidStorageEntry(PathBuf),

    #[error("failed to read {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to write {path}: {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to create directory {path}: {source}")]
    CreateDirectory {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to remove directory {path}: {source}")]
    RemoveDirectory {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to inspect {path}: {source}")]
    Metadata {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to move {from} to {to}: {source}")]
    Rename {
        from: PathBuf,
        to: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("invalid TOML in {path}: {source}")]
    Decode {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("failed to encode instance configuration: {0}")]
    Encode(#[from] toml::ser::Error),
}
