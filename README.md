# Konnex Subtensor

Konnex Subtensor is a fork of the Bittensor Subtensor chain.  
We are evolving it toward Konnex goals: robotics-native coordination, subnet-aware task execution, and verifiable physical work settlement.

Project docs: [docs.konnex.world](https://docs.konnex.world/)

## What We Are Building

Konnex targets a permissionless robotics AI market where:

- robot tasks are broadcast to competing AI providers,
- execution quality is verified by decentralized participants,
- final outcomes are settled on-chain with escrow-like guarantees,
- consensus links task identity, subnet identity, and settlement events.

In this repository, we focus on **chain/runtime responsibilities** (node, pallets, consensus-integrated logic).  
SDK/client integrations are handled in companion repositories.

## Quick Start

### Prerequisites

- Rust toolchain (as pinned by this repository)
- Git
- Linux or macOS recommended for node operations

### Build

```bash
cargo build --release
```

### Run local dev node

```bash
cargo run --release -- --dev
```

### Useful checks

```bash
cargo check --workspace
cargo test --workspace --no-run
```

## KNX Windows-Specific Requirements and Fixes

On Windows, `cargo check --workspace` may fail if extra native tools are missing.

### 1) Install Protocol Buffers compiler (`protoc`)

```powershell
winget install --id Google.Protobuf -e
```

If `protoc` is not found after install, restart the terminal.  
Quick check:

```powershell
protoc --version
```

If restart did not help, add `protoc` to visible PATH manually.

Current session:

```powershell
$protoc = Get-ChildItem "$env:LOCALAPPDATA\Microsoft\WinGet\Packages" -Recurse -Filter protoc.exe -ErrorAction SilentlyContinue | Select-Object -First 1 -ExpandProperty FullName
$env:PATH = "$($protoc | Split-Path);$env:PATH"
$env:PROTOC = $protoc
protoc --version
```

Persist for next shells:

```powershell
setx PROTOC "$env:PROTOC"
setx PATH "$(($env:PROTOC | Split-Path));$env:PATH"
```

### 2) Install LLVM/Clang (`libclang` for bindgen)

```powershell
winget install --id LLVM.LLVM -e
```

If build scripts still cannot find `libclang.dll`, set:

```powershell
$env:LIBCLANG_PATH = "C:\Program Files\LLVM\bin"
$env:PATH = "$env:LIBCLANG_PATH;$env:PATH"
```

Then retry:

```powershell
cargo check --workspace
```

### 3) Typical errors and meaning

- `Could not find protoc` -> install protobuf compiler and reopen shell.
- `Unable to find libclang` -> install LLVM and set `LIBCLANG_PATH`.
- `Blocking waiting for file lock on package cache` -> another `cargo` process is already running; wait or stop it.

## Repository Scope

Key areas in this repository:

- `node/` - node service and CLI
- `runtime/` - chain runtime
- `pallets/` - domain modules
- `docs/` - protocol and development docs

## Fork Context

This codebase started as a fork of Bittensor Subtensor and is being adapted incrementally.  
We intentionally prefer small, reviewable changes over large rewrites.

## Acknowledgments

- Upstream Bittensor Subtensor maintainers and contributors
- Substrate ecosystem maintainers
