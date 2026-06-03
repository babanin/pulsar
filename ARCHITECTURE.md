# Pulsar Architecture

## Overview

Pulsar is an orchestrator and process supervisor for OpenVPN-over-Cloak VPN connections. It is not a VPN implementation itself -- it coordinates existing `openvpn` and `ck-client` binaries, managing their lifecycle, configuration, and logs. The core responsibilities are:

- Importing VPN profiles from AmneziaVPN backups or manual configuration files
- Storing and retrieving profiles on disk
- Starting, monitoring, and stopping child processes in the correct order
- Detecting the runtime platform and resolving bundled binaries

## Architecture

```
pulsar
  Profile Import (amnezia, manual)
  Profile Storage (~/.config/pulsar/profiles/)
  Process Supervision (tokio child processes)
  Logging (tracing + file output)
  bundled/ck-client
  bundled/openvpn
```

## Module Structure

| Module | Path | Description |
|---|---|---|
| Entry point | `src/main.rs` | Parses CLI args via `clap`, initializes `tracing`, dispatches to `app::run` inside a `#[tokio::main]` runtime |
| Library root | `src/lib.rs` | Re-exports all public modules (`cli`, `connector`, `error`, `import`, `logging`, `platform`, `process`, `profile`) for integration tests |
| CLI definitions | `src/cli.rs` | Clap-derived `Cli` struct and `Commands` / `ProfileCommands` enums |
| Command dispatch | `src/app.rs` | `run()` function that matches on `Commands` and delegates to handler functions; contains all business logic for `doctor`, `profile`, `connect`, `disconnect`, `status` |
| Error types | `src/error.rs` | `PulsarError` enum (thiserror) with variants for profile, binary, platform, process, connection, and IO errors; `Result<T>` type alias |
| Profile models | `src/profile/model.rs` | `Profile`, `ProfileData`, `ProtocolType`, `CloakConfig` structs with serde; `CloakConfig::validate()` checks required fields; `Profile::sanitize_name()` validates names |
| Profile storage | `src/profile/store.rs` | `ProfileStore` handles file I/O under `~/.config/pulsar/profiles/{name}/`; stores `profile.json`, `openvpn.ovpn`, `cloak.json` |
| Import dispatcher | `src/import/mod.rs` | `ImportSource` enum and `import()` function that dispatches to the appropriate parser |
| Amnezia parser | `src/import/amnezia.rs` | Parses AmneziaVPN backup JSON, handles double-encoded `Servers/serversList`, locates `amnezia-openvpn-cloak` container, extracts cloak config and openvpn config |
| Manual import | `src/import/manual.rs` | Reads `.ovpn` and `cloak.json` files from disk, validates cloak config, infers remote host/port |
| Connector trait | `src/connector/mod.rs` | `Connector` trait (async start/stop/status), `ConnectionStatus` enum, `create_connector()` factory function |
| OpenVPN+Cloak | `src/connector/openvpn_cloak.rs` | `OpenVpnCloakConnector`: starts `ck-client`, polls `127.0.0.1:{port}` until TCP accepts, then starts `openvpn`; supervised via `ProcessSupervisor` |
| Process supervisor | `src/process/supervisor.rs` | `ManagedProcess` wraps a tokio `Child`; `ProcessSupervisor` manages a collection, kills in reverse-order (OpenVPN first, then Cloak) |
| Platform detection | `src/platform/mod.rs` | `Platform` enum (macOS x86_64, macOS aarch64, Linux x86_64), `resolve_binaries()` locates bundled binaries next to the executable, `check_binary_executable()` verifies execute permission |
| Logging setup | `src/logging.rs` | Configures `tracing-subscriber` with `EnvFilter`; supports `--verbose` flag and optional file output to `~/.config/pulsar/logs/pulsar.log` |

## Data Flow: Profile Import

### AmneziaVPN Import

```
User: pulsar profile import-amnezia --name home --file backup.backup
  |
  v
app::import_amnezia()
  - Sanitize profile name
  - Check profile doesn't already exist
  - Call import::import(ImportSource::AmneziaBackup { path })
      |
      v
    amnezia::import_amnezia_backup()
      - Parse outer JSON -> AmneziaBackup
      - Extract "Servers/serversList" (double-encoded: string containing JSON)
      - Parse inner JSON -> Vec<AmneziaServer>
      - Find container where container == "amnezia-openvpn-cloak"
      - Parse cloak.last_config -> CloakRawConfig -> CloakConfig
        (PascalCase serde: RemoteHost, RemotePort, PublicKey, UID, etc.)
      - Validate CloakConfig (required fields: PublicKey, UID, RemoteHost, RemotePort, ProxyMethod)
      - Parse openvpn.last_config -> OpenVpnLastConfig -> extract raw .ovpn text
      - Extract remote host/port from .ovpn text (regex on "remote HOST PORT")
      - Build Profile and ProfileData
  |
  v
ProfileStore::save()
  - Create ~/.config/pulsar/profiles/{name}/
  - Write profile.json, cloak.json, openvpn.ovpn
```

### Manual Import

```
User: pulsar profile import --name home --ovpn client.ovpn --cloak cloak.json
  |
  v
app::import_manual()
  - Call import::import(ImportSource::Manual { ovpn_path, cloak_path })
      |
      v
    manual::import_manual()
      - Read .ovpn file from disk
      - Read and parse cloak.json -> CloakConfig
      - Validate CloakConfig
      - Extract remote host/port
  |
  v
ProfileStore::save()
```

## Data Flow: Connect

```
User: sudo pulsar connect home
  |
  v
app::connect()
  - ProfileStore::load("home") -> Profile
  - ProfileStore::load_openvpn_config / load_cloak_config
  - Platform::current() -> Platform
  - resolve_binaries(platform) -> BinaryPaths (or user-specified system binaries)
  - create_connector(OpenvpnCloak, ...) -> OpenVpnCloakConnector
  |
  v
OpenVpnCloakConnector::start()
  - Set status to Connecting
  - Start ck-client -c cloak.json -l 1194
  - Poll 127.0.0.1:1194 with TCP connect (100ms interval, 30s timeout)
  - On accept: Cloak is ready
  - Start openvpn --config openvpn.ovpn
  - Set status to Connected
  - wait_any() on ProcessSupervisor (blocks until a process exits)
  - On exit: kill_all() in reverse order (OpenVPN first, then Cloak)
```

### Shutdown

When any managed process exits unexpectedly, or on Ctrl+C:

```
ProcessSupervisor::kill_all()
  - Iterate processes in reverse insertion order
  - OpenVPN is stopped first (added last, reversed = first killed)
  - Cloak is stopped second
  - Each process receives SIGTERM, then wait() for clean exit
```

## Connector Trait (Extensibility)

```rust
#[async_trait]
pub trait Connector: Send + Sync {
    fn protocol_type(&self) -> ProtocolType;
    async fn start(&self) -> Result<()>;
    async fn stop(&self) -> Result<()>;
    fn status(&self) -> ConnectionStatus;
}
```

The `create_connector()` factory maps `ProtocolType` variants to concrete `Connector` implementations. To add a new protocol (e.g., XRay VLESS):

1. Add a variant to `ProtocolType` in `src/profile/model.rs`
2. Implement the `Connector` trait in a new `src/connector/xray_vless.rs` module
3. Add the match arm in `create_connector()` in `src/connector/mod.rs`
4. Add an importer in `src/import/`

## Importer Extensibility

```rust
pub enum ImportSource {
    AmneziaBackup { path: String },
    AmneziaFileContents { contents: String },
    Manual { ovpn_path: String, cloak_path: String },
}
```

To add a new import format:

1. Add a variant to `ImportSource` in `src/import/mod.rs`
2. Add a match arm in `import()` that dispatches to a new parser
3. Create a new `src/import/{format}.rs` module implementing the parser
4. Add a corresponding CLI subcommand in `src/cli.rs` and handler in `src/app.rs`

## Configuration Model

### Profile

```rust
pub struct Profile {
    pub name: String,
    pub protocol_type: ProtocolType,  // "openvpn-cloak"
    pub created_at: String,             // RFC 3339 timestamp
    pub remote_host: String,
    pub remote_port: u16,
    pub local_cloak_port: u16,          // default: 1194
}
```

### ProtocolType

```rust
pub enum ProtocolType {
    OpenvpnCloak,  // serialized as "openvpn-cloak" (kebab-case)
}
```

### CloakConfig

```rust
pub struct CloakConfig {
    // serde(rename_all = "PascalCase") -- fields serialize as
    // BrowserSig, EncryptionMethod, NumConn, ProxyMethod,
    // PublicKey, RemoteHost, RemotePort, ServerName,
    // StreamTimeout, Transport, UID
}
```

`CloakConfig::validate()` ensures `PublicKey`, `UID`, `RemoteHost`, `RemotePort`, and `ProxyMethod` are non-empty. Returns `PulsarError::InvalidCloakConfig` with a list of missing fields.

### On-disk layout

```
~/.config/pulsar/
  profiles/
    home/
      profile.json      # serialized Profile
      cloak.json        # serialized CloakConfig (PascalCase)
      openvpn.ovpn      # raw OpenVPN config text
  logs/
    pulsar.log
```

## Error Handling

All errors are represented by the `PulsarError` enum, derived via `thiserror::Error`:

- `ProfileNotFound`, `ProfileAlreadyExists`, `InvalidProfileName` -- profile CRUD errors
- `BinaryMissing`, `BinaryNotExecutable` -- bundled binary resolution errors
- `PlatformNotSupported` -- unsupported OS/arch
- `InvalidAmneziaProfile`, `InvalidCloakConfig`, `MissingCloakConfig`, `MissingOpenVpnConfig` -- import and configuration errors
- `ProcessStartFailed`, `ProcessExited` -- child process errors
- `ConnectionFailed`, `NotConnected`, `AlreadyConnected` -- connection state errors
- `CloakReadyTimeout` -- Cloak failed to accept TCP within 30 seconds
- `ConfigDirNotWritable` -- disk I/O for config directory
- `Io`, `Json`, `Regex` -- wrapper errors with `#[from]`

The `Result<T>` type alias is used throughout. No `unwrap()` on fallible operations in non-test code; no panics on user-facing paths. All error messages are actionable (e.g., `Cloak did not become ready at {host}:{port} within {timeout}s` rather than "timeout").

## Testing Approach

- **Unit tests**: each module contains `#[cfg(test)] mod tests` with focused test cases (see `src/profile/store.rs`, `src/import/amnezia.rs`, `src/platform/mod.rs`)
- **Integration tests**: `tests/integration.rs` exercises end-to-end flows: import Amnezia backup, import manual configs, save/load profiles, validate CloakConfig
- **Test fixtures**: `tests/fixtures/` contains sample files (`amnezia-openvpn-cloak.backup`, `client.ovpn`, `cloak.json`)
- **No root-requiring tests**: all tests use `tempfile::tempdir()` for profile storage and fixture files for import; nothing requires system privileges or live network access
- Profile store tests use `ProfileStore::with_base_dir()` with temporary directories