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
