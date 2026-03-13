#!/usr/bin/env python3
"""
Simulate pallet_subtensor block emission (get_block_emission_for_issuance shape).

Each block (default 12s):
  emission_rao = f(total_issuance_rao, total_supply_rao, default_block_emission_rao)
  total_issuance_rao += emission_rao

Token in plots: KNX (same 1e-9 smallest unit as upstream "rao"; values are not on-chain exact).

Usage:
  python3 emission_simulation.py -o logs/emission_simulation_2y.png
  python3 emission_simulation.py --days 365 --scenarios 90000000000,114155000000,140000000000
"""

from __future__ import annotations

import argparse
import csv
import math
import sys
from pathlib import Path

TOKEN = "KNX"
# Smallest units (same scale as repo "rao"): 1 KNX = 1e9 units
TOTAL_SUPPLY_UNITS = 1_000_000_000_000_000_000  # 1B KNX
INITIAL_ISSUANCE_UNITS = 700_000_000_000_000_000  # 700M KNX at genesis
BLOCK_SECONDS = 12
SECONDS_PER_DAY = 86_400

# ~114e9: rough target to mint ~15% of cap over 2y from 70% genesis (see hand calc in chat).
DEFAULT_SCENARIOS = "90_000_000_000,114_155_000_000,140_000_000_000".replace("_", "")


def block_emission_units(issuance: int, total_supply: int, default_block_emission: int) -> int:
    """Mirror pallets/subtensor/src/coinbase/block_emission.rs (float approx)."""
    if issuance >= total_supply:
        return 0
    if total_supply <= 0:
        return 0
    ratio = issuance / total_supply
    inner = 1.0 - ratio
    if inner <= 0.0:
        return 0
    x = 1.0 / inner
    if x <= 0.0:
        return 0
    residual = math.log2(x)
    floored = int(math.floor(residual))
    if floored < 0:
        floored = 0
    multiplier = 2.0**floored
    pct = 1.0 / multiplier
    raw = pct * float(default_block_emission)
    if raw <= 0.0:
        return 0
    return int(raw)


def simulate_days(
    days: float,
    initial_issuance: int,
    total_supply: int,
    default_block_emission: int,
    block_seconds: int,
) -> tuple[list[float], list[float], list[float]]:
    """
    Returns:
      days_axis — days (t=0 is genesis),
      total_supply_millions_knx — (genesis + minted) / 1e6 KNX (axis: 700 = 700M KNX),
      per_block_mint_knx — mint in this block in whole KNX (float),
    """
    step_sec = block_seconds
    n_blocks = int((days * SECONDS_PER_DAY) / step_sec)
    if n_blocks < 1:
        n_blocks = 1

    issuance = initial_issuance
    cumulative_units = 0
    days_axis: list[float] = [0.0]
    total_supply_millions_knx: list[float] = [(initial_issuance / 1e9) / 1e6]
    per_block_mint_knx: list[float] = []

    for i in range(n_blocks):
        em = block_emission_units(issuance, total_supply, default_block_emission)
        cumulative_units += em
        issuance = min(issuance + em, total_supply)

        t = (i + 1) * step_sec / SECONDS_PER_DAY
        days_axis.append(t)
        total_whole_knx = (initial_issuance + cumulative_units) / 1e9
        total_supply_millions_knx.append(total_whole_knx / 1e6)
        per_block_mint_knx.append(em / 1e9)

    return days_axis, total_supply_millions_knx, per_block_mint_knx


def subsample(xs: list[float], ys: list[float], max_points: int) -> tuple[list[float], list[float]]:
    n = len(xs)
    if n <= max_points:
        return xs, ys
    step = max(1, (n + max_points - 1) // max_points)
    return xs[::step], ys[::step]


def main() -> int:
    parser = argparse.ArgumentParser(description=f"Simulate {TOKEN} emission vs DefaultBlockEmission")
    parser.add_argument("--days", type=float, default=730.0, help="Simulation horizon in days")
    parser.add_argument(
        "--initial-rao",
        type=int,
        default=INITIAL_ISSUANCE_UNITS,
        dest="initial_units",
        help="Starting TotalIssuance (smallest units)",
    )
    parser.add_argument(
        "--total-supply-rao",
        type=int,
        default=TOTAL_SUPPLY_UNITS,
        dest="total_supply_units",
        help="Cap TotalSupply (smallest units)",
    )
    parser.add_argument(
        "--scenarios",
        type=str,
        default=DEFAULT_SCENARIOS,
        help="Comma-separated DefaultBlockEmission (smallest units; curve scales from this ref)",
    )
    parser.add_argument(
        "-o",
        "--output",
        type=Path,
        default=None,
        help="Output PNG (default: logs/emission_simulation.png)",
    )
    parser.add_argument(
        "--max-plot-points",
        type=int,
        default=25_000,
        help="Subsample for PNG",
    )
    args = parser.parse_args()

    scenarios = [int(x.strip()) for x in args.scenarios.split(",") if x.strip()]

    repo_root = Path(__file__).resolve().parent
    logs_dir = repo_root / "logs"
    logs_dir.mkdir(parents=True, exist_ok=True)
    out_png = args.output or (logs_dir / "emission_simulation.png")

    series: dict[int, tuple[list[float], list[float], list[float]]] = {}
    for dbe in scenarios:
        series[dbe] = simulate_days(
            args.days,
            args.initial_units,
            args.total_supply_units,
            dbe,
            BLOCK_SECONDS,
        )

    cap_millions = (args.total_supply_units / 1e9) / 1e6
    initial_millions = (args.initial_units / 1e9) / 1e6
    ratio0 = args.initial_units / args.total_supply_units
    cap_label = f"TotalSupply cap ({cap_millions:,.0f}M {TOKEN})"

    try:
        import matplotlib.pyplot as plt
        from matplotlib.ticker import FuncFormatter
    except ImportError:
        csv_path = logs_dir / "emission_simulation.csv"
        max_len = max(len(series[s][0]) for s in scenarios)
        fieldnames = ["idx", "day"] + [f"total_supply_millions_{TOKEN}_dbe_{s}" for s in scenarios]
        with csv_path.open("w", newline="") as f:
            w = csv.DictWriter(f, fieldnames=fieldnames)
            w.writeheader()
            for i in range(max_len):
                row: dict[str, float | int] = {"idx": i}
                days0 = series[scenarios[0]][0]
                row["day"] = days0[i] if i < len(days0) else days0[-1]
                for s in scenarios:
                    _, tot_m, _ = series[s]
                    row[f"total_supply_millions_{TOKEN}_dbe_{s}"] = tot_m[i] if i < len(tot_m) else tot_m[-1]
                w.writerow(row)
        print(
            "matplotlib not installed; wrote CSV:\n"
            f"  {csv_path}\n"
            "Install: apt install python3-matplotlib",
            file=sys.stderr,
        )
        return 0

    fig, (ax_top, ax_bot) = plt.subplots(2, 1, figsize=(10, 9), sharex=True, gridspec_kw={"height_ratios": [2, 1]})

    for dbe in scenarios:
        days, tot_millions, per_block_knx = series[dbe]
        d_top, y_top = subsample(days, tot_millions, args.max_plot_points)
        d_pb, y_pb = subsample(days[1:], per_block_knx, args.max_plot_points)

        label = f"dbe={dbe:_} units ({dbe / 1e9:.2f} {TOKEN}/block @100% curve ref)"
        ax_top.plot(d_top, y_top, label=label, linewidth=1.6)
        ax_bot.plot(d_pb, y_pb, label=label, linewidth=1.0, alpha=0.9)

    ax_top.axhline(cap_millions, color="black", linestyle="--", linewidth=1.0, label=cap_label)
    ax_top.set_ylabel(f"Total supply (millions of {TOKEN})\n= genesis + minted this sim")
    ax_top.yaxis.set_major_formatter(FuncFormatter(lambda x, _p: f"{x:,.0f}"))
    ax_top.grid(True, alpha=0.3)
    ax_top.legend(loc="lower right", fontsize=7)
    ax_top.set_title(
        f"{TOKEN} emission schedule (approx.)\n"
        f"12s blocks; genesis = {initial_millions:,.0f}M {TOKEN} ({ratio0:.1%} of cap); "
        f"cap = {cap_millions:,.0f}M {TOKEN}\n"
        "Top: total supply (M KNX). Bottom: mint per block (KNX) — steps when floor(log₂) changes."
    )

    ax_bot.set_xlabel("Days")
    ax_bot.set_ylabel(f"Mint this block ({TOKEN})")
    ax_bot.grid(True, alpha=0.3)
    ax_bot.legend(loc="upper right", fontsize=7)

    fig.tight_layout()
    out_png.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(out_png, dpi=150)
    print(f"Wrote {out_png}")
    for dbe in scenarios:
        _, tot_m, _ = series[dbe]
        minted_m = tot_m[-1] - initial_millions
        print(
            f"  dbe={dbe:_}: end total {tot_m[-1]:,.2f}M {TOKEN} "
            f"(minted from sim ~{minted_m:,.2f}M {TOKEN} vs genesis)"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
