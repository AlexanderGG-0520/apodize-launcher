# Apodize Launcher — Initial Architecture

Status: Draft  
Scope: Phase 0–2  
Language: Rust  
Targets: CLI / TUI

## 1. Purpose

Apodize Launcher is a lightweight and fast Minecraft launcher designed primarily for terminal users.

The project provides both a CLI and a TUI. Both interfaces must expose the same core capabilities and use the same underlying application logic.

Apodize does not attempt to replace existing terminal tools when those tools already solve a general-purpose problem well. The launcher should focus on functionality that is specifically tied to Minecraft instance management, launching, content management, authentication, and related lifecycle operations.

## 2. Design Principles

1. The CLI and TUI must use the same Core API.
2. The Core must not depend on CLI- or TUI-specific types, rendering logic, or terminal UI libraries.
3. Prefer existing OS and CLI tools for generic terminal operations that are not Minecraft-specific.
4. Configuration should use human-readable and directly editable formats where practical.
5. Persisted data formats must include an explicit `schema_version` from the first public version.
6. External APIs and filesystem persistence must be separated from the domain model.
7. The Core must not call `println!` or `eprintln!` for user-facing output.
8. Tests should not depend on live network services unless explicitly marked as integration tests.
9. Dependencies should be added only when they provide clear value over the Rust standard library.
10. CLI and TUI feature parity is a project requirement, not a best-effort goal.
11. Lightweight startup, low memory overhead, and fast execution are first-class requirements.
12. New features should be rejected when they duplicate generic terminal functionality without strong Minecraft-specific justification.

## 3. Responsibility Boundary

A feature belongs in Apodize when the launcher needs to understand, persist, or coordinate Minecraft-specific state.

Examples that belong in Apodize:

- Minecraft instance management
- Minecraft version and loader selection
- Java selection and compatibility management
- JVM arguments
- Environment variables tied to an instance
- Pre-launch and post-exit commands
- Mod, resource pack, and shader installation
- Modpack import
- Microsoft account management
- Skin management
- Minecraft launch orchestration
- mclo.gs integration

Examples that should normally remain external:

- General text editing → `$EDITOR`
- Generic file browsing → `ls`, `eza`, `find`, etc.
- Generic log searching → `grep`, `rg`, etc.
- Generic process monitoring → `ps`, `btop`, etc.
- Java installation itself → system package manager or other external tooling
- Generic configuration file editing → `$EDITOR`

The key question is not merely whether something can be done in a terminal, but whether Apodize needs to understand that operation as part of Minecraft state or lifecycle management.

## 4. Initial Workspace Layout

```text
apodize/
├── Cargo.toml
├── rustfmt.toml
├── .gitignore
├── README.md
├── docs/
│   └── architecture.md
└── crates/
    ├── apodize-core/
    │   ├── Cargo.toml
    │   └── src/
    │       ├── lib.rs
    │       ├── error.rs
    │       └── instance/
    │           ├── mod.rs
    │           ├── model.rs
    │           ├── repository.rs
    │           └── service.rs
    ├── apodize-cli/
    │   ├── Cargo.toml
    │   └── src/
    │       └── main.rs
    └── apodize-tui/
        ├── Cargo.toml
        └── src/
            └── main.rs
```

The Core contains all application and domain logic. The CLI and TUI are frontends over that Core.

```text
                 apodize-core
                /            \
               /              \
      apodize-cli          apodize-tui
```

The CLI must not depend on the TUI.  
The TUI must not depend on the CLI.

## 5. Initial Workspace Configuration

The root `Cargo.toml` should manage shared package metadata and dependency versions.

```toml
[workspace]
resolver = "3"
members = [
    "crates/apodize-core",
    "crates/apodize-cli",
    "crates/apodize-tui",
]

[workspace.package]
edition = "2024"
license = "LGPL-2.1-only"
repository = "https://github.com/AlexanderGG-0520/apodize-launcher"

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
thiserror = "2"
```

Additional dependencies should be introduced only when the corresponding functionality is implemented.

## 6. Initial Domain Model

The first domain to implement is instance management.

The initial model should remain small, but it should avoid putting all configuration into one monolithic struct.

```rust
pub struct Instance {
    pub id: InstanceId,
    pub name: String,
    pub minecraft: MinecraftConfig,
    pub java: JavaConfig,
    pub launch: LaunchConfig,
}
```

### 6.1 Instance ID

Instance names must not be used as persistent identifiers.

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InstanceId(String);
```

Renaming an instance must not require changing its identity or filesystem location.

UUID v7 or ULID are preferred candidates for generated IDs.

Example storage layout:

```text
instances/
└── 019c.../
    └── instance.toml
```

### 6.2 Minecraft Configuration

```rust
pub struct MinecraftConfig {
    pub version: String,
    pub loader: Option<LoaderConfig>,
}
```

The loader representation can remain minimal until loader support is implemented.

### 6.3 Java Configuration

```rust
pub struct JavaConfig {
    pub executable: Option<PathBuf>,
    pub jvm_args: Vec<String>,
}
```

`None` for `executable` means that no explicit Java installation has been pinned to the instance.

### 6.4 Launch Configuration

```rust
pub struct LaunchConfig {
    pub environment: BTreeMap<String, String>,
    pub commands: CustomCommands,
}
```

`BTreeMap` is preferred over `HashMap` for persisted environment variables because deterministic ordering produces cleaner TOML output and smaller Git diffs.

### 6.5 Custom Commands

Commands should be represented as an executable plus arguments rather than as an implicit shell command string.

```rust
#[derive(Debug, Clone, Default)]
pub struct CustomCommands {
    pub before_launch: Vec<CommandSpec>,
    pub after_exit: Vec<CommandSpec>,
}

#[derive(Debug, Clone)]
pub struct CommandSpec {
    pub program: String,
    pub args: Vec<String>,
}
```

Example:

```toml
[[launch.commands.before_launch]]
program = "echo"
args = ["Starting Minecraft"]
```

This avoids making shell parsing and quoting semantics part of the default command model.

If shell execution is ever added, it should be an explicit mode rather than an implicit behavior.

## 7. Persistence Model

The persisted representation must be separated from the domain model.

Do not serialize the main domain structs directly merely because Serde makes it convenient.

Instead, define version-specific storage structs such as:

```rust
struct InstanceFileV1 {
    schema_version: u32,
    instance: InstanceSection,
    minecraft: MinecraftSection,
    java: JavaSection,
    launch: LaunchSection,
}
```

Conversion flow:

```text
instance.toml
    ↓ deserialize
InstanceFileV1
    ↓ validate / convert
Instance
```

This prevents internal refactoring from unintentionally changing the public configuration format.

## 8. `instance.toml` Schema v1

Initial proposal:

```toml
schema_version = 1

[instance]
id = "019c..."
name = "Survival"

[minecraft]
version = "1.21.8"

[java]
executable = "/usr/bin/java"
jvm_args = [
    "-Xms1G",
    "-Xmx4G",
]

[launch.environment]
MESA_SHADER_CACHE_DIR = "/tmp/minecraft-shader-cache"

[[launch.commands.before_launch]]
program = "echo"
args = ["Launching Survival"]

[[launch.commands.after_exit]]
program = "echo"
args = ["Minecraft exited"]
```

Future loader support can extend the schema without modifying the meaning of existing fields.

Example:

```toml
[minecraft.loader]
kind = "fabric"
version = "0.17.2"
```

## 9. Schema Versioning and Migration

Every persisted instance file must contain:

```toml
schema_version = 1
```

The loader must reject unsupported future schema versions rather than silently misinterpreting them.

Future migration should follow an explicit path:

```text
v1 → v2 → v3
```

Migrations should be deterministic and testable.

Configuration files become part of the application's compatibility surface as soon as users begin relying on them. Schema migration is therefore an architectural concern from the beginning, not a later cleanup task.

## 10. Repository Boundary

Instance persistence should sit behind a narrow repository abstraction.

```rust
pub trait InstanceRepository {
    fn create(&self, instance: &Instance) -> Result<(), InstanceError>;

    fn get(
        &self,
        id: &InstanceId,
    ) -> Result<Option<Instance>, InstanceError>;

    fn list(&self) -> Result<Vec<Instance>, InstanceError>;

    fn delete(&self, id: &InstanceId) -> Result<(), InstanceError>;
}
```

The normal implementation will initially be filesystem-backed:

```rust
pub struct FileInstanceRepository {
    root: PathBuf,
}
```

Tests may use an in-memory repository.

The abstraction must remain narrow. Apodize should not create a generic virtual filesystem abstraction unless a real requirement emerges.

## 11. Application Service

Frontends should not manipulate repository implementations directly.

```rust
pub struct InstanceService<R> {
    repository: R,
}
```

Example API shape:

```rust
impl<R: InstanceRepository> InstanceService<R> {
    pub fn create(
        &self,
        request: CreateInstanceRequest,
    ) -> Result<Instance, InstanceError> {
        // ...
    }
}
```

Expected flow:

```text
CLI / TUI input
      ↓
Application Request
      ↓
InstanceService
      ↓
InstanceRepository
```

User-facing rendering belongs in the frontend, not the Core.

## 12. Initial CLI Surface

The first vertical slice should implement only instance CRUD.

```text
apodize instance create <name> --minecraft <version>
apodize instance list
apodize instance show <id>
apodize instance remove <id>
```

Example:

```text
$ apodize instance create survival --minecraft 1.21.8

Created instance:
  survival
  Minecraft 1.21.8

$ apodize instance list

ID            NAME       VERSION
019c...       survival   1.21.8
```

The application does not need to launch Minecraft at this stage.

## 13. Feature Parity Rule

All user-facing capabilities must be represented in the Core independently of frontend presentation.

A simple feature matrix should be maintained as the project grows.

| Capability | Core | CLI | TUI |
|---|---:|---:|---:|
| Instance create | Yes | Yes | Yes |
| Instance list | Yes | Yes | Yes |
| Instance show | Yes | Yes | Yes |
| Instance remove | Yes | Yes | Yes |
| Java detection | Planned | Planned | Planned |
| Mod install | Planned | Planned | Planned |
| Account management | Planned | Planned | Planned |

A feature is not considered complete until both frontends can expose it, unless the capability is explicitly frontend-specific presentation behavior.

## 14. Error Handling

Core errors should be typed and meaningful.

Expected categories include:

- `InstanceError`
- `JavaError`
- `DownloadError`
- `LaunchError`
- `AuthenticationError`
- `ModpackError`

The Core returns structured errors. The CLI and TUI decide how those errors are rendered to users.

The Core must not emit user-facing text directly through stdout or stderr.

## 15. Testing Requirements

The first implementation must include tests for at least:

- Creating an instance
- Persisting an instance
- Loading the same instance again
- Listing instances
- Removing an instance
- Round-trip serialization and deserialization
- Invalid TOML
- Unsupported `schema_version`
- Missing required Minecraft version
- Duplicate instance ID

Critical round-trip property:

```text
Instance
  ↓ encode
instance.toml
  ↓ decode
Instance
```

The decoded instance should be semantically equivalent to the original instance.

External API tests should use fixtures rather than live endpoints wherever practical.

## 16. Continuous Integration Baseline

CI should be introduced from the beginning.

Minimum checks:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

Dependency auditing should also be introduced early, with tools such as `cargo-deny`, because dependency growth is itself a technical-debt risk for a lightweight launcher.

## 17. Explicit Non-Goals for Phase 0–2

Do not implement yet:

- Minecraft launching
- Microsoft authentication
- Skin management
- Modrinth or CurseForge integration
- Mod installation
- Resource pack installation
- Shader installation
- Modpack import
- mclo.gs integration
- Java download/install management
- Full TUI screens
- Generic file manager features
- Generic text editor features
- Generic process monitoring

These are intentionally deferred until the core instance model and persistence boundaries are stable.

## 18. Phase 0–2 Completion Criteria

Phase 0–2 is complete when:

1. The Rust workspace builds successfully.
2. `apodize-core` owns the instance domain and persistence interfaces.
3. `instance.toml` v1 is explicitly defined and versioned.
4. Instance files can be created, loaded, listed, and removed safely.
5. The CLI exposes instance CRUD without containing domain logic.
6. Round-trip and invalid-data tests pass.
7. CI enforces formatting, Clippy, and workspace tests.
8. No TUI-specific dependency exists in `apodize-core`.
9. No CLI-specific dependency exists in `apodize-core`.
10. No live network service is required for the initial test suite.

## 19. Planned Development Order

```text
Phase 0  Design principles
Phase 1  Cargo workspace and Core boundaries
Phase 2  Instance schema, persistence, migration baseline, CLI CRUD
Phase 3  Java detection and Java compatibility model
Phase 4  Minecraft version metadata
Phase 5  Download manager
Phase 6  Minecraft launch pipeline
Phase 7  Mods / resource packs / shaders
Phase 8  Modpack import
Phase 9  Microsoft account and skin management
Phase 10 mclo.gs integration
Phase 11 TUI implementation over stable Core APIs
```

The order may change when requirements become clearer, but changes should preserve the architectural boundaries defined above.

## 20. Architectural Warning Signs

The following should trigger a design review:

- Business logic appears inside a CLI command handler.
- TUI code directly accesses instance files.
- CLI and TUI implement the same operation separately.
- `apodize-core` imports `clap`, `ratatui`, or `crossterm`.
- Domain structs are changed solely to satisfy TOML layout requirements.
- Generic helpers accumulate into a large `utils.rs` module.
- Live HTTP requests become necessary for ordinary unit tests.
- A dependency is added for functionality already handled cleanly by the standard library.
- A feature reproduces generic terminal tooling without a Minecraft-specific state or lifecycle requirement.
- Persisted files are changed without a schema migration plan.

These are not absolute prohibitions, but they are strong indicators that the architecture may be drifting away from the project's goals.
