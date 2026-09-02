use std::{collections::BTreeMap, fmt, path::PathBuf, str::FromStr};

use uuid::Uuid;

use crate::InstanceError;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct InstanceId(Uuid);

impl InstanceId {
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    pub fn parse(value: &str) -> Result<Self, InstanceError> {
        Uuid::parse_str(value)
            .map(Self)
            .map_err(|_| InstanceError::InvalidId(value.to_owned()))
    }
}

impl Default for InstanceId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for InstanceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for InstanceId {
    type Err = InstanceError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instance {
    id: InstanceId,
    name: String,
    minecraft: MinecraftConfig,
    java: JavaConfig,
    launch: LaunchConfig,
}

impl Instance {
    pub fn new(
        name: impl Into<String>,
        minecraft_version: impl Into<String>,
    ) -> Result<Self, InstanceError> {
        Self::from_parts(
            InstanceId::new(),
            name.into(),
            MinecraftConfig::new(minecraft_version)?,
            JavaConfig::default(),
            LaunchConfig::default(),
        )
    }

    pub(crate) fn from_parts(
        id: InstanceId,
        name: String,
        minecraft: MinecraftConfig,
        java: JavaConfig,
        launch: LaunchConfig,
    ) -> Result<Self, InstanceError> {
        if name.trim().is_empty() {
            return Err(InstanceError::EmptyName);
        }

        Ok(Self {
            id,
            name,
            minecraft,
            java,
            launch,
        })
    }

    #[must_use]
    pub fn id(&self) -> &InstanceId {
        &self.id
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn minecraft(&self) -> &MinecraftConfig {
        &self.minecraft
    }

    #[must_use]
    pub fn java(&self) -> &JavaConfig {
        &self.java
    }

    #[must_use]
    pub fn launch(&self) -> &LaunchConfig {
        &self.launch
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MinecraftConfig {
    version: String,
    loader: Option<LoaderConfig>,
}

impl MinecraftConfig {
    pub fn new(version: impl Into<String>) -> Result<Self, InstanceError> {
        let version = version.into();
        if version.trim().is_empty() {
            return Err(InstanceError::EmptyMinecraftVersion);
        }

        Ok(Self {
            version,
            loader: None,
        })
    }

    pub(crate) fn with_loader(
        version: impl Into<String>,
        loader: Option<LoaderConfig>,
    ) -> Result<Self, InstanceError> {
        let mut config = Self::new(version)?;
        config.loader = loader;
        Ok(config)
    }

    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    #[must_use]
    pub fn loader(&self) -> Option<&LoaderConfig> {
        self.loader.as_ref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoaderConfig {
    kind: String,
    version: String,
}

impl LoaderConfig {
    #[must_use]
    pub fn new(kind: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            version: version.into(),
        }
    }

    #[must_use]
    pub fn kind(&self) -> &str {
        &self.kind
    }

    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct JavaConfig {
    executable: Option<PathBuf>,
    jvm_args: Vec<String>,
}

impl JavaConfig {
    #[must_use]
    pub(crate) fn from_parts(executable: Option<PathBuf>, jvm_args: Vec<String>) -> Self {
        Self {
            executable,
            jvm_args,
        }
    }

    #[must_use]
    pub fn executable(&self) -> Option<&PathBuf> {
        self.executable.as_ref()
    }

    #[must_use]
    pub fn jvm_args(&self) -> &[String] {
        &self.jvm_args
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LaunchConfig {
    environment: BTreeMap<String, String>,
    commands: CustomCommands,
}

impl LaunchConfig {
    #[must_use]
    pub(crate) fn from_parts(
        environment: BTreeMap<String, String>,
        commands: CustomCommands,
    ) -> Self {
        Self {
            environment,
            commands,
        }
    }

    #[must_use]
    pub fn environment(&self) -> &BTreeMap<String, String> {
        &self.environment
    }

    #[must_use]
    pub fn commands(&self) -> &CustomCommands {
        &self.commands
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CustomCommands {
    before_launch: Vec<CommandSpec>,
    after_exit: Vec<CommandSpec>,
}

impl CustomCommands {
    #[must_use]
    pub(crate) fn from_parts(
        before_launch: Vec<CommandSpec>,
        after_exit: Vec<CommandSpec>,
    ) -> Self {
        Self {
            before_launch,
            after_exit,
        }
    }

    #[must_use]
    pub fn before_launch(&self) -> &[CommandSpec] {
        &self.before_launch
    }

    #[must_use]
    pub fn after_exit(&self) -> &[CommandSpec] {
        &self.after_exit
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSpec {
    program: String,
    args: Vec<String>,
}

impl CommandSpec {
    #[must_use]
    pub fn new(program: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            program: program.into(),
            args,
        }
    }

    #[must_use]
    pub fn program(&self) -> &str {
        &self.program
    }

    #[must_use]
    pub fn args(&self) -> &[String] {
        &self.args
    }
}
