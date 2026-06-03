# AGENTS.md

## Build Commands

- Build: `cargo build`
- Build release: `cargo build --release`
- Run tests: `cargo test`
- Run all tests (unit + integration): `cargo test --all`
- Check compilation: `cargo check`

## Lint and Format

- **Clippy** (all warnings as errors): `cargo clippy -- -D warnings`
- **Format check**: `cargo fmt -- --check`
- **Apply format**: `cargo fmt`

Always run both clippy and fmt check before committing. CI enforces both.

## CI Checks

On every push to main/master and on pull requests:
1. `cargo fmt -- --check` must pass
2. `cargo clippy -- -D warnings` must pass
3. `cargo test --all` must pass

## Project Structure

```
src/
  main.rs              - Entry point, tokio runtime
  lib.rs               - Library root, re-exports all modules
  cli.rs               - Clap derive CLI definitions
  app.rs               - Command dispatch and business logic
  error.rs             - PulsarError enum (thiserror)
  logging.rs           - tracing-subscriber init
  profile/
    mod.rs             - Module root
    model.rs           - Profile, ProtocolType, CloakConfig data models
    store.rs           - File-based profile CRUD in ~/.config/pulsar/profiles/
  import/
    mod.rs             - ImportSource enum and dispatch
    amnezia.rs         - AmneziaVPN backup JSON parser
    manual.rs          - Direct .ovpn + cloak.json import
  connector/
    mod.rs             - Connector trait, ConnectionStatus, factory
    openvpn_cloak.rs   - OpenVPN+Cloak process orchestrator
  process/
    mod.rs             - Module root
    supervisor.rs      - ManagedProcess and ProcessSupervisor
  platform/
    mod.rs             - Platform enum, binary resolution relative to exe
tests/
  integration.rs       - End-to-end import/store tests
  fixtures/
    amnezia-openvpn-cloak.backup  - Redacted AmneziaVPN backup
    client.ovpn       - Minimal OpenVPN config fixture
    cloak.json        - Cloak config fixture
bundled/
  macos-x86_64/        - Placeholder stubs (replace with real binaries)
  macos-aarch64/       - Placeholder stubs
  linux-x86_64/        - Placeholder stubs
```

## Key Design Decisions

- Pulsar is an orchestrator only. It does NOT implement VPN protocols.
- Bundled binaries are resolved relative to the executable, not system PATH.
- The `Connector` trait enables adding new protocols (e.g., XRay VLESS) without modifying core logic.
- The `ImportSource` enum enables adding new import formats.
- Profile names are restricted to alphanumeric, hyphens, and underscores.
- AmneziaVPN backups use double-encoded JSON (`Servers/serversList` is a JSON string inside JSON).
- Cloak JSON uses PascalCase field names with a special `UID` rename.

## AmneziaVPN Backup Format

The `.backup` file is JSON with double-encoded nested structures:
1. Top level: `Servers/serversList` is a JSON string containing a server array
2. Each server has `containers[]` with `"container": "amnezia-openvpn-cloak"`
3. Container has `cloak.last_config` (JSON string) and `openvpn.last_config` (JSON string)
4. `openvpn.last_config` contains `config` field with the raw .ovpn content
5. Cloak JSON uses PascalCase: `BrowserSig`, `EncryptionMethod`, `PublicKey`, `UID` (uppercase)

## Error Handling

- All errors use `PulsarError` enum with `thiserror` derives
- No panics. All fallible operations return `Result<T>` or `PulsarError`
- Actionable error messages: "Profile not found: home", "Bundled binary missing: bundled/macos-aarch64/ck-client"

## Testing Guidelines

- Unit tests live inside module files under `#[cfg(test)] mod tests`
- Integration tests go in `tests/integration.rs`
- Use `tempfile::tempdir()` for profile store tests
- Do NOT write tests requiring root privileges
- Fixtures in `tests/fixtures/` use redacted data (IP 203.0.113.1, placeholder keys)