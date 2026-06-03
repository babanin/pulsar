# Pulsar

A lightweight CLI orchestrator for OpenVPN-over-Cloak VPN connections. Pulsar replaces the client-side functionality of AmneziaVPN for OpenVPN-over-Cloak setups by importing profiles, extracting configurations, and managing the lifecycle of bundled `ck-client` and `openvpn` binaries.

## What Pulsar Does

- Imports AmneziaVPN backup profiles and extracts OpenVPN + Cloak configurations
- Imports OpenVPN and Cloak configurations manually from separate files
- Launches bundled `ck-client` and `openvpn` binaries
- Supervises child processes and ensures clean teardown on disconnect or Ctrl+C
- Polls the local Cloak proxy until it is ready before starting OpenVPN
- Verifies the runtime environment (platform support, binary availability, config directory)

## What Pulsar Does Not Do

- Provide a GUI -- Pulsar is a terminal-only tool
- Implement VPN protocols -- it delegates to `ck-client` and `openvpn`
- Provision or manage VPN servers
- Support WireGuard or other VPN protocols

## Supported Platforms

| Platform | Status |
|---|---|
| macOS x86_64 (Intel) | Supported |
| macOS arm64 (Apple Silicon) | Supported |
| Linux x86_64 | Supported |

Bundled binaries for each platform are expected alongside the `pulsar` executable under `bundled/<platform>/`.

## Installation

### From Source

Requirements: Rust toolchain (1.70+), Cargo.

```sh
cargo build --release
```

The binary is produced at `target/release/pulsar`. Place it in a directory that also contains the `bundled/` folder with the appropriate `ck-client` and `openvpn` binaries for your platform.

## Usage

### Verify Environment

```sh
pulsar doctor
```

Checks platform support, bundled binary availability and executability, and config directory writability.

### Import a Profile

From an AmneziaVPN export file:

```sh
pulsar profile import-amnezia --name home --file ./amnezia-export.ovpn
```

From separate OpenVPN and Cloak config files:

```sh
pulsar profile import --name home --ovpn ./client.ovpn --cloak ./cloak.json
```

### List Profiles

```sh
pulsar profile list
```

### Show Profile Details

```sh
pulsar profile show home
```

### Connect

```sh
sudo pulsar connect home
```

To use system-installed `ck-client` and `openvpn` instead of the bundled binaries:

```sh
sudo pulsar connect home --use-system-binaries
```

### Disconnect

```sh
pulsar disconnect
```

### Check Status

```sh
pulsar status
```

### Verbose Output

Append `-v` or `--verbose` to any command for debug-level logging.

```sh
pulsar -v connect home
```

## Connect Flow

1. **Start Cloak** -- Launch `ck-client -c <cloak.json> -l <port>` as a supervised child process
2. **Poll local proxy** -- Repeatedly attempt a TCP connection to `127.0.0.1:1194` until Cloak is ready (timeout: 30s, interval: 100ms)
3. **Start OpenVPN** -- Launch `openvpn --config <openvpn.ovpn>` as a supervised child process
4. **Stream logs** -- stdout and stderr from both processes are captured and logged
5. **Handle shutdown** -- On Ctrl+C or process exit, the supervisor kills all processes in reverse launch order

## Configuration Storage

Profiles are stored under:

```
~/.config/pulsar/profiles/
```

Each profile is a directory containing three files:

| File | Description |
|---|---|
| `profile.json` | Profile metadata (name, protocol, remote host/port, timestamps) |
| `openvpn.ovpn` | OpenVPN client configuration |
| `cloak.json` | Cloak client configuration |

Profile names may only contain alphanumeric characters, hyphens, and underscores.

## Building from Source

```sh
git clone https://github.com/anomalyco/pulsar.git
cd pulsar
cargo build --release
```

The release binary will be at `target/release/pulsar`.

## Running Tests

```sh
cargo test
```

## Project Structure

```
pulsar/
  bundled/                  Platform-specific ck-client and openvpn binaries
    macos-aarch64/
    macos-x86_64/
    linux-x86_64/
  src/
    main.rs                Entry point and CLI dispatch
    app.rs                 Command handlers (doctor, connect, disconnect, etc.)
    cli.rs                 Clap-based CLI definition
    connector/
      mod.rs               Connector trait and factory
      openvpn_cloak.rs     OpenVPN-over-Cloak connection orchestrator
    error.rs               PulsarError enum
    import/
      mod.rs               Import dispatch (Amnezia vs manual)
      amnezia.rs           AmneziaVPN backup parser
      manual.rs            Manual config file importer
    logging.rs             Tracing subscriber initialization
    platform/
      mod.rs               Platform detection and binary resolution
    process/
      mod.rs               Process management module
      supervisor.rs        ManagedProcess and ProcessSupervisor
    profile/
      mod.rs               Profile module root
      model.rs             Profile, CloakConfig, ProfileData types
      store.rs             filesystem-backed profile store
  tests/
    integration.rs         Integration tests for import and store
```

## License

MIT