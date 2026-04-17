# Konnex — scope for audit (v2)

Черновик: список **будет дополняться** по мере согласования чартера. Ниже: escrow + цепочка **coinbase** (эмиссия блока → шаг блока → раздача).

**Ветка репозитория, под которую зафиксирован этот документ:** **`main`**.

---

## Аудируемый коммит (снапшот) и upstream baseline

### Konnex (что сравниваем / что отдаём аудиторам)

**`e8dd5fb284c53dcb6ec8fea7d0c6e238ec8dea25`** (`e8dd5fb28`) — *Add matplotlib script to generate Konnex escrow flowcharts.*

При смене ветки или тега для аудита **обновите этот раздел** на актуальный `git rev-parse HEAD`.

### Upstream Bittensor / Subtensor (точка форка)

**Репозиторий:** [opentensor/subtensor](https://github.com/opentensor/subtensor) (нода и паллеты сети Bittensor / Subtensor).

**Baseline (коммит upstream на момент ветвления форка):**  
**`e9bbb6134984b1f9f63f5e55408faad7628eb059`**

Дифф форка относительно baseline (см. также внутренний черновик `ignored/konnex-diff-for-audit.md`, если есть в клоне):

```bash
git diff e9bbb6134984b1f9f63f5e55408faad7628eb059..HEAD
```

Если в вашем клоне другое имя remotes, baseline можно перепроверить: `git merge-base HEAD <upstream-remote>/main` (или ветка, от которой делали fork).

---

## 1. Скоуп на аудит (обязательные исходники)

Инструмент: **`cloc` 2.04** (`cloc --by-file …`).

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

### 1.2 `pallets/subtensor/src/coinbase/`

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

### 1.3 Итого обязательный скоуп (§1.1 + §1.2)

| | Total | Blank | Comment | Code |
|--|------:|------:|--------:|-----:|
| **Sum** | **1891** | **159** | **372** | **1360** |

---

## 2. Вспомогательные файлы на аудит (тесты и моки)

Исключаются из основного бюджета строк §1, но перечислены для контекста и регрессий.

| Path | Role | Total | Blank | Comment | Code |
|------|------|------:|------:|--------:|-----:|
| `pallets/konnex-escrow/src/mock.rs` | Test runtime | 62 | 8 | 0 | 54 |
| `pallets/konnex-escrow/src/tests.rs` | Escrow unit tests | 669 | 40 | 16 | 613 |
