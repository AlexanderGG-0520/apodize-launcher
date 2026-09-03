use std::{
    collections::BTreeMap,
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::InstanceError;

use super::model::{
    CommandSpec, CustomCommands, Instance, InstanceId, JavaConfig, LaunchConfig, LoaderConfig,
    MinecraftConfig,
};

pub const CURRENT_INSTANCE_SCHEMA_VERSION: u32 = 1;
const INSTANCE_FILE_NAME: &str = "instance.toml";
const TEMP_INSTANCE_FILE_NAME: &str = "instance.toml.tmp";

pub trait InstanceRepository {
    fn create(&self, instance: &Instance) -> Result<(), InstanceError>;
    fn get(&self, id: &InstanceId) -> Result<Option<Instance>, InstanceError>;
    fn list(&self) -> Result<Vec<Instance>, InstanceError>;
    fn delete(&self, id: &InstanceId) -> Result<(), InstanceError>;
}

#[derive(Debug, Clone)]
pub struct FileInstanceRepository {
    root: PathBuf,
}

impl FileInstanceRepository {
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    fn instance_dir(&self, id: &InstanceId) -> PathBuf {
        self.root.join(id.to_string())
    }

    fn instance_file(&self, id: &InstanceId) -> PathBuf {
        self.instance_dir(id).join(INSTANCE_FILE_NAME)
    }

    fn ensure_root(&self) -> Result<(), InstanceError> {
        fs::create_dir_all(&self.root).map_err(|source| InstanceError::CreateDirectory {
            path: self.root.clone(),
            source,
        })
    }

    fn load_path(
        &self,
        path: &Path,
        expected_id: Option<&InstanceId>,
    ) -> Result<Instance, InstanceError> {
        let metadata = fs::symlink_metadata(path).map_err(|source| InstanceError::Metadata {
            path: path.to_path_buf(),
            source,
        })?;
        if metadata.file_type().is_symlink() {
            return Err(InstanceError::SymlinkStorage(path.to_path_buf()));
        }
        if !metadata.is_file() {
            return Err(InstanceError::InvalidStorageEntry(path.to_path_buf()));
        }

        let content = fs::read_to_string(path).map_err(|source| InstanceError::Read {
            path: path.to_path_buf(),
            source,
        })?;

        decode_instance(path, &content, expected_id)
    }
}

impl InstanceRepository for FileInstanceRepository {
    fn create(&self, instance: &Instance) -> Result<(), InstanceError> {
        let encoded = encode_instance(instance)?;
        self.ensure_root()?;

        let instance_dir = self.instance_dir(instance.id());
        match fs::create_dir(&instance_dir) {
            Ok(()) => {}
            Err(source) if source.kind() == ErrorKind::AlreadyExists => {
                return Err(InstanceError::AlreadyExists(instance.id().to_string()));
            }
            Err(source) => {
                return Err(InstanceError::CreateDirectory {
                    path: instance_dir,
                    source,
                });
            }
        }

        let file_path = instance_dir.join(INSTANCE_FILE_NAME);
        let temp_path = instance_dir.join(TEMP_INSTANCE_FILE_NAME);

        if let Err(source) = fs::write(&temp_path, encoded) {
            let _ = fs::remove_dir(&instance_dir);
            return Err(InstanceError::Write {
                path: temp_path,
                source,
            });
        }

        if let Err(source) = fs::rename(&temp_path, &file_path) {
            let _ = fs::remove_file(&temp_path);
            let _ = fs::remove_dir(&instance_dir);
            return Err(InstanceError::Rename {
                from: temp_path,
                to: file_path,
                source,
            });
        }

        Ok(())
    }

    fn get(&self, id: &InstanceId) -> Result<Option<Instance>, InstanceError> {
        let instance_dir = self.instance_dir(id);
        let metadata = match fs::symlink_metadata(&instance_dir) {
            Ok(metadata) => metadata,
            Err(source) if source.kind() == ErrorKind::NotFound => return Ok(None),
            Err(source) => {
                return Err(InstanceError::Metadata {
                    path: instance_dir,
                    source,
                });
            }
        };

        if metadata.file_type().is_symlink() {
            return Err(InstanceError::SymlinkStorage(instance_dir));
        }
        if !metadata.is_dir() {
            return Err(InstanceError::InvalidStorageEntry(instance_dir));
        }

        self.load_path(&self.instance_file(id), Some(id)).map(Some)
    }

    fn list(&self) -> Result<Vec<Instance>, InstanceError> {
        let entries = match fs::read_dir(&self.root) {
            Ok(entries) => entries,
            Err(source) if source.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
            Err(source) => {
                return Err(InstanceError::Read {
                    path: self.root.clone(),
                    source,
                });
            }
        };

        let mut instances = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|source| InstanceError::Read {
                path: self.root.clone(),
                source,
            })?;
            let entry_path = entry.path();
            let file_type = entry
                .file_type()
                .map_err(|source| InstanceError::Metadata {
                    path: entry_path.clone(),
                    source,
                })?;

            if file_type.is_symlink() {
                return Err(InstanceError::SymlinkStorage(entry_path));
            }
            if !file_type.is_dir() {
                continue;
            }

            let instance_file = entry_path.join(INSTANCE_FILE_NAME);
            if !instance_file.exists() {
                continue;
            }

            let directory_name = entry.file_name();
            let directory_name = directory_name
                .to_str()
                .ok_or_else(|| InstanceError::InvalidStorageEntry(entry_path.clone()))?;
            let expected_id = InstanceId::parse(directory_name)?;
            instances.push(self.load_path(&instance_file, Some(&expected_id))?);
        }

        instances.sort_by(|left, right| {
            left.name()
                .cmp(right.name())
                .then_with(|| left.id().cmp(right.id()))
        });
        Ok(instances)
    }

    fn delete(&self, id: &InstanceId) -> Result<(), InstanceError> {
        let instance_dir = self.instance_dir(id);
        let metadata = match fs::symlink_metadata(&instance_dir) {
            Ok(metadata) => metadata,
            Err(source) if source.kind() == ErrorKind::NotFound => {
                return Err(InstanceError::NotFound(id.to_string()));
            }
            Err(source) => {
                return Err(InstanceError::Metadata {
                    path: instance_dir,
                    source,
                });
            }
        };

        if metadata.file_type().is_symlink() {
            return Err(InstanceError::SymlinkStorage(instance_dir));
        }
        if !metadata.is_dir() {
            return Err(InstanceError::InvalidStorageEntry(instance_dir));
        }

        fs::remove_dir_all(&instance_dir).map_err(|source| InstanceError::RemoveDirectory {
            path: instance_dir,
            source,
        })
    }
}

fn encode_instance(instance: &Instance) -> Result<String, InstanceError> {
    let file = InstanceFileV1::from(instance);
    Ok(toml::to_string_pretty(&file)?)
}

fn decode_instance(
    path: &Path,
    content: &str,
    expected_id: Option<&InstanceId>,
) -> Result<Instance, InstanceError> {
    let header: SchemaHeader = toml::from_str(content).map_err(|source| InstanceError::Decode {
        path: path.to_path_buf(),
        source,
    })?;

    if header.schema_version != CURRENT_INSTANCE_SCHEMA_VERSION {
        return Err(InstanceError::UnsupportedSchemaVersion {
            found: header.schema_version,
            supported: CURRENT_INSTANCE_SCHEMA_VERSION,
        });
    }

    let file: InstanceFileV1 = toml::from_str(content).map_err(|source| InstanceError::Decode {
        path: path.to_path_buf(),
        source,
    })?;
    let instance = Instance::try_from(file)?;

    if let Some(expected_id) = expected_id
        && instance.id() != expected_id
    {
        return Err(InstanceError::StorageIdMismatch {
            directory_id: expected_id.to_string(),
            file_id: instance.id().to_string(),
        });
    }

    Ok(instance)
}

#[derive(Debug, Deserialize)]
struct SchemaHeader {
    schema_version: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct InstanceFileV1 {
    schema_version: u32,
    instance: InstanceSection,
    minecraft: MinecraftSection,
    #[serde(default)]
    java: JavaSection,
    #[serde(default)]
    launch: LaunchSection,
}

#[derive(Debug, Serialize, Deserialize)]
struct InstanceSection {
    id: String,
    name: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct MinecraftSection {
    version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    loader: Option<LoaderSection>,
}

#[derive(Debug, Serialize, Deserialize)]
struct LoaderSection {
    kind: String,
    version: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct JavaSection {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    executable: Option<PathBuf>,
    #[serde(default)]
    jvm_args: Vec<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct LaunchSection {
    #[serde(default)]
    environment: BTreeMap<String, String>,
    #[serde(default)]
    commands: CommandsSection,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct CommandsSection {
    #[serde(default)]
    before_launch: Vec<CommandSection>,
    #[serde(default)]
    after_exit: Vec<CommandSection>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CommandSection {
    program: String,
    #[serde(default)]
    args: Vec<String>,
}

impl From<&Instance> for InstanceFileV1 {
    fn from(instance: &Instance) -> Self {
        Self {
            schema_version: CURRENT_INSTANCE_SCHEMA_VERSION,
            instance: InstanceSection {
                id: instance.id().to_string(),
                name: instance.name().to_owned(),
            },
            minecraft: MinecraftSection {
                version: instance.minecraft().version().to_owned(),
                loader: instance.minecraft().loader().map(|loader| LoaderSection {
                    kind: loader.kind().to_owned(),
                    version: loader.version().to_owned(),
                }),
            },
            java: JavaSection {
                executable: instance.java().executable().cloned(),
                jvm_args: instance.java().jvm_args().to_vec(),
            },
            launch: LaunchSection {
                environment: instance.launch().environment().clone(),
                commands: CommandsSection {
                    before_launch: instance
                        .launch()
                        .commands()
                        .before_launch()
                        .iter()
                        .map(CommandSection::from)
                        .collect(),
                    after_exit: instance
                        .launch()
                        .commands()
                        .after_exit()
                        .iter()
                        .map(CommandSection::from)
                        .collect(),
                },
            },
        }
    }
}

impl From<&CommandSpec> for CommandSection {
    fn from(command: &CommandSpec) -> Self {
        Self {
            program: command.program().to_owned(),
            args: command.args().to_vec(),
        }
    }
}

impl TryFrom<InstanceFileV1> for Instance {
    type Error = InstanceError;

    fn try_from(file: InstanceFileV1) -> Result<Self, Self::Error> {
        let loader = file
            .minecraft
            .loader
            .map(|loader| LoaderConfig::new(loader.kind, loader.version));
        let minecraft = MinecraftConfig::with_loader(file.minecraft.version, loader)?;
        let java = JavaConfig::from_parts(file.java.executable, file.java.jvm_args);
        let commands = CustomCommands::from_parts(
            file.launch
                .commands
                .before_launch
                .into_iter()
                .map(|command| CommandSpec::new(command.program, command.args))
                .collect(),
            file.launch
                .commands
                .after_exit
                .into_iter()
                .map(|command| CommandSpec::new(command.program, command.args))
                .collect(),
        );
        let launch = LaunchConfig::from_parts(file.launch.environment, commands);

        Instance::from_parts(
            InstanceId::parse(&file.instance.id)?,
            file.instance.name,
            minecraft,
            java,
            launch,
        )
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    fn repository() -> (TempDir, FileInstanceRepository) {
        let temp = TempDir::new().expect("create temporary directory");
        let repository = FileInstanceRepository::new(temp.path().join("instances"));
        (temp, repository)
    }

    #[test]
    fn create_and_load_round_trip() {
        let (_temp, repository) = repository();
        let instance = Instance::new("Survival", "1.21.8").expect("create instance");

        repository.create(&instance).expect("persist instance");
        let loaded = repository
            .get(instance.id())
            .expect("load instance")
            .expect("instance exists");

        assert_eq!(loaded, instance);
    }

    #[test]
    fn list_returns_instances_in_deterministic_order() {
        let (_temp, repository) = repository();
        let beta = Instance::new("beta", "1.21.8").expect("create beta");
        let alpha = Instance::new("alpha", "1.20.1").expect("create alpha");

        repository.create(&beta).expect("persist beta");
        repository.create(&alpha).expect("persist alpha");

        let listed = repository.list().expect("list instances");
        assert_eq!(listed, vec![alpha, beta]);
    }

    #[test]
    fn delete_removes_instance() {
        let (_temp, repository) = repository();
        let instance = Instance::new("Disposable", "1.21.8").expect("create instance");
        repository.create(&instance).expect("persist instance");

        repository.delete(instance.id()).expect("delete instance");

        assert!(
            repository
                .get(instance.id())
                .expect("query instance")
                .is_none()
        );
    }

    #[test]
    fn duplicate_instance_id_is_rejected() {
        let (_temp, repository) = repository();
        let instance = Instance::new("Duplicate", "1.21.8").expect("create instance");
        repository.create(&instance).expect("persist first copy");

        let error = repository
            .create(&instance)
            .expect_err("duplicate id must fail");

        assert!(matches!(error, InstanceError::AlreadyExists(_)));
    }

    #[test]
    fn invalid_toml_is_rejected() {
        let (_temp, repository) = repository();
        let id = InstanceId::new();
        let dir = repository.instance_dir(&id);
        fs::create_dir_all(&dir).expect("create instance directory");
        fs::write(dir.join(INSTANCE_FILE_NAME), "this is not = valid = toml")
            .expect("write invalid file");

        let error = repository.get(&id).expect_err("invalid TOML must fail");
        assert!(matches!(error, InstanceError::Decode { .. }));
    }

    #[test]
    fn unsupported_schema_version_is_rejected_before_full_decode() {
        let (_temp, repository) = repository();
        let id = InstanceId::new();
        let dir = repository.instance_dir(&id);
        fs::create_dir_all(&dir).expect("create instance directory");
        fs::write(
            dir.join(INSTANCE_FILE_NAME),
            format!(
                "schema_version = 999\nunknown_future_field = true\n[instance]\nid = \"{id}\"\n"
            ),
        )
        .expect("write future schema file");

        let error = repository
            .get(&id)
            .expect_err("future schema version must fail");
        assert!(matches!(
            error,
            InstanceError::UnsupportedSchemaVersion {
                found: 999,
                supported: CURRENT_INSTANCE_SCHEMA_VERSION
            }
        ));
    }

    #[test]
    fn missing_minecraft_version_is_rejected() {
        let (_temp, repository) = repository();
        let id = InstanceId::new();
        let dir = repository.instance_dir(&id);
        fs::create_dir_all(&dir).expect("create instance directory");
        fs::write(
            dir.join(INSTANCE_FILE_NAME),
            format!(
                "schema_version = 1\n[instance]\nid = \"{id}\"\nname = \"Broken\"\n[minecraft]\n"
            ),
        )
        .expect("write incomplete instance file");

        let error = repository
            .get(&id)
            .expect_err("missing Minecraft version must fail");
        assert!(matches!(error, InstanceError::Decode { .. }));
    }

    #[test]
    fn storage_directory_and_file_id_must_match() {
        let (_temp, repository) = repository();
        let directory_id = InstanceId::new();
        let file_instance = Instance::new("Mismatch", "1.21.8").expect("create instance");
        let dir = repository.instance_dir(&directory_id);
        fs::create_dir_all(&dir).expect("create instance directory");
        fs::write(
            dir.join(INSTANCE_FILE_NAME),
            encode_instance(&file_instance).expect("encode instance"),
        )
        .expect("write instance file");

        let error = repository
            .get(&directory_id)
            .expect_err("id mismatch must fail");
        assert!(matches!(error, InstanceError::StorageIdMismatch { .. }));
    }
}
