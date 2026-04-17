# Konnex — audit scope (v2)

Draft: this list **may grow** as the audit charter is finalized. In scope: **Konnex escrow** + Subtensor **`epoch`** (`run_epoch.rs`) + **coinbase** chain (per-block emission → block step → distribution).

**Repository branch this document refers to:** **`main`**.

---

## Audit snapshot and upstream baseline

### Konnex (fork under review)

**`e8dd5fb284c53dcb6ec8fea7d0c6e238ec8dea25`** (`e8dd5fb28`)

| | |
|--|--|
| **GitLab** | https://git.konnex.world/ |
| **Repository** | https://git.konnex.world/vlads/konnex-chain-subtensor |
| **Branch** | `main` |

When the audited branch or tag changes, **update this section** (e.g. `git rev-parse HEAD`).

### Upstream Bittensor / Subtensor (fork point)

**Repository:** [opentensor/subtensor](https://github.com/opentensor/subtensor) (Bittensor / Subtensor node and pallets).

**Baseline (upstream commit at fork time):**  
**`e9bbb6134984b1f9f63f5e55408faad7628eb059`**

Fork diff against baseline:

```bash
git diff e9bbb6134984b1f9f63f5e55408faad7628eb059..HEAD
```

If your remote names differ, re-check with: `git merge-base HEAD <upstream-remote>/main` (or the branch you forked from).

---

## 1. Mandatory audit scope (source files)

Tool: **`cloc` 2.04** (`cloc --by-file …`).

### 1.1 `pallets/konnex-escrow/src/`

```
pallets/konnex-escrow/src/
├── lib.rs
└── weights.rs
```

| Path | Total | Blank | Comment | Code |
|------|------:|------:|--------:|-----:|
| `pallets/konnex-escrow/src/lib.rs` | 445 | 43 | 75 | 327 |
| `pallets/konnex-escrow/src/weights.rs` | 42 | 5 | 2 | 35 |
| **Subtotal** | **487** | **48** | **77** | **362** |

### 1.2 `pallets/subtensor/src/epoch/`

```
pallets/subtensor/src/epoch/
└── run_epoch.rs
```

| Path | Total | Blank | Comment | Code |
|------|------:|------:|--------:|-----:|
| `pallets/subtensor/src/epoch/run_epoch.rs` | 1396 | 189 | 303 | 904 |
| **Subtotal** | **1396** | **189** | **303** | **904** |

### 1.3 `pallets/subtensor/src/coinbase/`

```
pallets/subtensor/src/coinbase/
├── block_emission.rs
├── block_step.rs
└── run_coinbase.rs
```

| Path | Total | Blank | Comment | Code |
|------|------:|------:|--------:|-----:|
| `pallets/subtensor/src/coinbase/block_emission.rs` | 159 | 12 | 56 | 91 |
| `pallets/subtensor/src/coinbase/block_step.rs` | 259 | 10 | 45 | 204 |
| `pallets/subtensor/src/coinbase/run_coinbase.rs` | 986 | 89 | 194 | 703 |
| **Subtotal** | **1404** | **111** | **295** | **998** |

### 1.4 Mandatory scope total (§1.1 + §1.2 + §1.3)

| | Total | Blank | Comment | Code |
|--|------:|------:|--------:|-----:|
| **Sum** | **3287** | **348** | **675** | **2264** |

---

## 2. Supporting files (tests and mocks)

Out of the §1 line budget, but listed for context and regressions.

| Path | Role | Total | Blank | Comment | Code |
|------|------|------:|------:|--------:|-----:|
| `pallets/konnex-escrow/src/mock.rs` | Test runtime | 62 | 8 | 0 | 54 |
| `pallets/konnex-escrow/src/tests.rs` | Escrow unit tests | 669 | 40 | 16 | 613 |
