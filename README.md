# iec104

Rust library for **[IEC 60870-5-104](https://en.wikipedia.org/wiki/IEC_60870-5#IEC_60870-5-104)** (IEC 104 over TCP): APDUs, ASDUs, type IDs, and async I/O on **Tokio**. The focus is **balanced** (station-to-station) communication as used in energy and automation SCADA.

**Status:** usable for client work, low-level servers, and the bundled RTU-style server layer, but the API and behavior are still evolving. Report issues and try the examples before relying on it in production.

[![crates.io](https://img.shields.io/crates/v/iec104.svg)](https://crates.io/crates/iec104)
[![docs.rs](https://docs.rs/iec104/badge.svg)](https://docs.rs/iec104)

## What this crate provides

| Area | Module | Role |
| ---- | ------ | ---- |
| **Client** | [`iec104::client`](https://docs.rs/iec104/latest/iec104/client/index.html) | Connect to a remote IEC 104 station, send commands, receive ASDUs via [`ClientCallback`](https://docs.rs/iec104/latest/iec104/client/trait.ClientCallback.html). Optional **TLS** is configured on [`ClientConfig`](https://docs.rs/iec104/latest/iec104/config/struct.ClientConfig.html) ([`TlsClientConfig`](https://docs.rs/iec104/latest/iec104/config/struct.TlsClientConfig.html)). |
| **Server (low level)** | [`iec104::server`](https://docs.rs/iec104/latest/iec104/server/index.html) | Accept TCP connections, run the 104 link layer (STARTDT / TESTFR / …), decode ingress ASDUs, and deliver them to your [`ServerCallback`](https://docs.rs/iec104/latest/iec104/server/trait.ServerCallback.html). You implement all application logic (interrogations, spontaneous data, command handling). TLS is optional via [`ServerConfig`](https://docs.rs/iec104/latest/iec104/config/struct.ServerConfig.html) / [`TlsServerConfig`](https://docs.rs/iec104/latest/iec104/config/struct.TlsServerConfig.html). |
| **RTU server (high level)** | [`iec104::rtu_server`](https://docs.rs/iec104/latest/iec104/rtu_server/index.html) | In-memory **typed points**, general and group interrogation, counter interrogation, pluggable **process commands** ([`RtuCommandHandler`](https://docs.rs/iec104/latest/iec104/rtu_server/trait.RtuCommandHandler.html)), and default **system ASDUs** (clock sync, test, read, reset process, etc.). Built on the low-level `Server`. |
| **Types & parsing** | [`iec104::types`](https://docs.rs/iec104/latest/iec104/types/index.html), [`iec104::asdu`](https://docs.rs/iec104/latest/iec104/asdu/index.html), [`iec104::types_id`](https://docs.rs/iec104/latest/iec104/types_id/index.html) | ASDU building blocks, information objects, and type IDs used by client and server paths. |

This is **IEC 60870-5-104 only** (not 101 serial profiles). File transfer, integrated totals beyond what the RTU layer models, and every optional standard feature are not goals for v1 completeness—check [`TypeId`](https://docs.rs/iec104/latest/iec104/types_id/enum.TypeId.html) and module docs for what is encoded and parsed today.

## Examples

Build and run from the repository root:

```bash
cargo run --example client
cargo run --example server
cargo run --example rtu_server
```

| Example | Description |
| ------- | ----------- |
| [`examples/client.rs`](examples/client.rs) | Minimal **master/client**: connect, start receiving, demonstrate sending a process command. |
| [`examples/server.rs`](examples/server.rs) | Minimal **slave/server** callback: accept connections and react to incoming ASDUs yourself. |
| [`examples/rtu_server.rs`](examples/rtu_server.rs) | **RTU-style station** with a documented point table (common address **47**), interrogation, counter interrogation, clock sync, and command presets—intended for learning and for the Python harness below. |

## Python client / integration harness

End-to-end checks against `examples/rtu_server` live under [`tests/`](tests/):

- [`tests/run_rtu_e2e.sh`](tests/run_rtu_e2e.sh) — builds the example, starts it, runs [`tests/test_client.py`](tests/test_client.py).
- The script expects the **[iec104-python](https://pypi.org/project/c104/)** module (`c104`) in the environment (for example the project `.venv`; see comments in the shell script).

The RTU example is aligned with that client for general interrogation, clock sync, counter interrogation, group interrogation, **C_TS_TA_1** test command, and process commands. Note: interoperability always depends on both stacks’ choices (type IDs, COTs, timeouts).

## Cargo features

| Feature | Default | Effect |
| ------- | ------- | ------ |
| `chrono` | **yes** | Conversions between IEC time tags ([`Cp24Time2a`](https://docs.rs/iec104/latest/iec104/types/time/struct.Cp24Time2a.html), [`Cp56Time2a`](https://docs.rs/iec104/latest/iec104/types/time/struct.Cp56Time2a.html)) and **chrono** `DateTime` (disable default features with `default-features = false` if you do not want the dependency). |

## Requirements

- **Rust** with support for the edition and features declared in `Cargo.toml` (currently **Edition 2024**).
- **Tokio** runtime for async networking.

## Limitations and unsupported areas

- **Work in progress:** public APIs and default RTU behavior may change between minor releases until the crate stabilises.
- **Coverage:** not every standard type ID or edge case is implemented or battle-tested; prefer explicit tests for your IO list.
- **Low-level `Server`:** does not implement an RTU process image—you get raw ASDUs and must send replies yourself.
- **Security:** TLS is available for transport; application-layer security (authentication, authorisation) is outside IEC 104 and not provided here.
- **101 / serial:** out of scope.

## Contributing

Contributions are welcome and **encouraged**. Please run tests and formatters locally before opening a pull request.

## Pre-commit

Hooks are optional but useful for consistent style:

1. Install [pre-commit](https://pre-commit.com) (package manager or `pip install --user pre-commit`).
2. `pre-commit autoupdate` — refresh hook revisions.
3. `pre-commit install` — enable hooks in this clone.
