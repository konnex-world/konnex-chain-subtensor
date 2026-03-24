#!/usr/bin/env python3
"""
Generate PNG flowcharts for the Konnex escrow FRAME pallet.

From repo root:
  python3 scripts/konnex_escrow_flowcharts.py

Outputs (logs/):
  konnex_escrow_1_swimlane.png
  konnex_escrow_2_state_machine.png
  konnex_escrow_3_linear_flow.png
  konnex_escrow_4_decision_tree.png

Edit CONFIG and the fig_* functions to adjust layout or copy.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

import matplotlib.pyplot as plt
from matplotlib.patches import FancyArrowPatch, FancyBboxPatch, Polygon
from matplotlib import patheffects as pe

REPO = Path(__file__).resolve().parent.parent
OUT_DIR = REPO / "logs"


@dataclass
class Config:
    dpi: int = 150
    face: str = "#0f1115"
    text: str = "#e8eaed"
    muted: str = "#9aa0a6"
    payer: str = "#5c9ded"
    payee: str = "#6bcf7f"
    resolver: str = "#c9a227"
    pallet: str = "#b388ff"
    edge_ok: str = "#81c995"
    edge_neutral: str = "#8ab4f8"
    edge_warn: str = "#f28b82"
    box_alpha: float = 0.92
    font: str = "DejaVu Sans"


CONFIG = Config()


def _rounded_box(
    ax,
    xy,
    w,
    h,
    text,
    fc,
    *,
    fontsize=9,
    lw=1.2,
    text_color=None,
):
    x, y = xy
    tc = text_color or CONFIG.text
    box = FancyBboxPatch(
        (x, y),
        w,
        h,
        boxstyle="round,pad=0.03,rounding_size=0.02",
        linewidth=lw,
        edgecolor=CONFIG.muted,
        facecolor=fc,
        alpha=CONFIG.box_alpha,
    )
    ax.add_patch(box)
    ax.text(
        x + w / 2,
        y + h / 2,
        text,
        ha="center",
        va="center",
        fontsize=fontsize,
        color=tc,
    )
    return {"x": x, "y": y, "w": w, "h": h, "cx": x + w / 2, "cy": y + h / 2, "top": y + h, "bot": y}


def _arrow_v(ax, x, y0, y1, **kw):
    p0, p1 = (x, y0), (x, y1)
    return _arrow(ax, p0, p1, **kw)


def _arrow(ax, p0, p1, *, color=None, lw=1.5, text="", fs=7, label_dx=0.0, label_dy=0.12):
    color = color or CONFIG.edge_neutral
    arr = FancyArrowPatch(
        p0,
        p1,
        arrowstyle="->",
        mutation_scale=12,
        linewidth=lw,
        color=color,
        connectionstyle="arc3,rad=0.02",
        shrinkA=6,
        shrinkB=6,
    )
    ax.add_patch(arr)
    if text:
        mx = (p0[0] + p1[0]) / 2 + label_dx
        my = (p0[1] + p1[1]) / 2 + label_dy
        ax.text(
            mx,
            my,
            text,
            ha="center",
            fontsize=fs,
            color=CONFIG.muted,
            path_effects=[pe.withStroke(linewidth=2.5, foreground=CONFIG.face)],
        )


def fig_swimlane() -> plt.Figure:
    """Horizontal phases + outcomes row; wide canvas to avoid overlap."""
    fig, ax = plt.subplots(figsize=(16, 9), facecolor=CONFIG.face)
    ax.set_facecolor(CONFIG.face)
    ax.set_xlim(0, 16)
    ax.set_ylim(0, 9)
    ax.axis("off")

    ax.text(
        8,
        8.35,
        "Konnex escrow — actors & flow (strict majority: 2·votes > n)",
        ha="center",
        fontsize=14,
        fontweight="bold",
        color=CONFIG.text,
    )
    ax.text(
        8,
        7.9,
        "FRAME pallet holds funds in its on-chain account (pot). Not an EVM contract.",
        ha="center",
        fontsize=9,
        color=CONFIG.muted,
    )

    # Lane labels (left)
    lanes = [
        ("Payer", CONFIG.payer, 6.15),
        ("Pallet (pot)", CONFIG.pallet, 4.55),
        ("Resolvers", CONFIG.resolver, 3.05),
        ("Payee", CONFIG.payee, 1.55),
    ]
    for name, color, y in lanes:
        ax.text(0.25, y + 0.35, name, fontsize=11, color=color, fontweight="bold", va="center")
        ax.axhline(y, color=CONFIG.muted, linewidth=0.5, alpha=0.3, xmin=0.02, xmax=0.99)

    h = 0.62
    # Row 1: main flow (same vertical band ~ y=6 for payer box center; stagger x)
    b_fund = _rounded_box(
        ax,
        (1.2, 5.85),
        2.5,
        h,
        "fund_escrow\n(amount, hash,\nresolvers, deadline)",
        CONFIG.payer,
        fontsize=8.5,
    )
    b_pot = _rounded_box(
        ax,
        (4.8, 5.85),
        2.8,
        h,
        "State: AwaitingRelease\n(funds in pot)",
        CONFIG.pallet,
        fontsize=8.5,
    )
    b_vote = _rounded_box(
        ax,
        (8.6, 5.85),
        2.8,
        h,
        "vote_on_escrow\n(one vote each)",
        CONFIG.resolver,
        fontsize=8.5,
    )

    _arrow(ax, (b_fund["x"] + b_fund["w"], b_fund["cy"]), (b_pot["x"], b_pot["cy"]), color=CONFIG.edge_ok, text="transfer", fs=7, label_dy=0.22)
    _arrow(ax, (b_pot["x"] + b_pot["w"], b_pot["cy"]), (b_vote["x"], b_vote["cy"]), color=CONFIG.edge_neutral, text="votes", fs=7, label_dy=0.22)

    # Payer timeout call — lower + curved path so it does not cross the vote row
    b_refund = _rounded_box(
        ax,
        (11.6, 2.95),
        3.35,
        h,
        "refund_after_deadline\n(payer, block > deadline,\nstill AwaitingRelease)",
        CONFIG.payer,
        fontsize=8,
    )
    arr_rt = FancyArrowPatch(
        (b_pot["cx"], b_pot["y"] + 0.05),
        (b_refund["cx"], b_refund["top"] + 0.06),
        arrowstyle="->",
        mutation_scale=11,
        linewidth=1.4,
        color=CONFIG.edge_warn,
        connectionstyle="arc3,rad=-0.42",
        shrinkA=8,
        shrinkB=8,
    )
    ax.add_patch(arr_rt)
    ax.text(9.2, 4.05, "if no majority", fontsize=7, color=CONFIG.muted)

    # Outcomes row — well separated
    b_rel = _rounded_box(ax, (2.0, 0.85), 3.2, 0.58, "Released\n(payee paid)", CONFIG.payee, fontsize=9)
    b_rej = _rounded_box(ax, (10.5, 0.85), 3.2, 0.58, "Refunded\n(reject quorum or timeout)", CONFIG.payer, fontsize=9)

    _arrow(ax, (b_pot["cx"], b_pot["bot"] - 0.02), (b_rel["cx"], b_rel["top"] + 0.04), color=CONFIG.edge_ok, text="approve\nquorum", fs=7, label_dx=-0.35, label_dy=0.18)
    _arrow(ax, (b_vote["cx"], b_vote["bot"] - 0.02), (b_rej["cx"], b_rej["top"] + 0.04), color=CONFIG.edge_warn, text="reject quorum\n(or timeout path)", fs=6.5, label_dx=0.4, label_dy=0.2)

    ax.text(
        8,
        0.22,
        "Events: EscrowFunded, EscrowVoted, EscrowReleased, EscrowRefunded",
        ha="center",
        fontsize=8,
        color=CONFIG.muted,
    )
    return fig


def fig_state_machine() -> plt.Figure:
    fig, ax = plt.subplots(figsize=(12, 10), facecolor=CONFIG.face)
    ax.set_facecolor(CONFIG.face)
    ax.set_xlim(0, 12)
    ax.set_ylim(0, 10)
    ax.axis("off")

    ax.text(
        6,
        9.45,
        "Konnex escrow — state machine",
        ha="center",
        fontsize=14,
        fontweight="bold",
        color=CONFIG.text,
    )

    b0 = _rounded_box(ax, (4.5, 8.05), 3.0, 0.58, "No escrow record", CONFIG.muted, fontsize=9)
    b1 = _rounded_box(ax, (3.9, 6.15), 4.2, 0.72, "AwaitingRelease\n(funds locked in pallet pot)", CONFIG.pallet, fontsize=10)
    b2 = _rounded_box(ax, (0.7, 3.35), 3.4, 0.68, "Released\n(payee receives amount)", CONFIG.payee, fontsize=9)
    b3 = _rounded_box(ax, (7.9, 3.35), 3.4, 0.68, "Refunded\n(payer receives amount)", CONFIG.payer, fontsize=9)

    _arrow_v(ax, b0["cx"], b0["bot"] - 0.02, b1["top"] + 0.02, color=CONFIG.edge_ok, text="fund_escrow", fs=8, label_dy=0.0)

    _arrow(ax, (b1["cx"] - 0.9, b1["bot"]), (b2["cx"], b2["top"] + 0.02), color=CONFIG.edge_ok, text="strict majority\napprove", fs=7.5, label_dx=-0.2, label_dy=0.15)
    _arrow(ax, (b1["cx"] + 0.9, b1["bot"]), (b3["cx"], b3["top"] + 0.02), color=CONFIG.edge_warn, text="strict majority\nreject", fs=7.5, label_dx=0.2, label_dy=0.15)

    arr = FancyArrowPatch(
        (b1["cx"], b1["bot"] - 0.08),
        (b3["cx"] - 0.35, b3["top"] + 0.08),
        arrowstyle="->",
        mutation_scale=11,
        linewidth=1.4,
        color=CONFIG.edge_neutral,
        connectionstyle="arc3,rad=0.35",
        shrinkA=8,
        shrinkB=8,
    )
    ax.add_patch(arr)
    ax.text(7.2, 4.95, "refund_after_deadline\n(no majority, block > deadline)", fontsize=7.5, color=CONFIG.muted)

    ax.text(
        6,
        2.05,
        "Only listed resolvers may vote; at most one ballot per resolver.",
        ha="center",
        fontsize=9,
        color=CONFIG.muted,
    )
    ax.text(
        6,
        1.55,
        "After Released or Refunded, further votes and timeout reclaim are rejected.",
        ha="center",
        fontsize=9,
        color=CONFIG.muted,
    )
    return fig


def fig_compact_flow() -> plt.Figure:
    fig, ax = plt.subplots(figsize=(9, 13), facecolor=CONFIG.face)
    ax.set_facecolor(CONFIG.face)
    ax.set_xlim(0, 9)
    ax.set_ylim(0, 13)
    ax.axis("off")

    ax.text(
        4.5,
        12.35,
        "Konnex escrow — linear scenario (simplified)",
        ha="center",
        fontsize=14,
        fontweight="bold",
        color=CONFIG.text,
    )

    steps = [
        ("1. Payer prepares subnet_id, task_id, payee,\namount, description_hash, resolvers, deadline", CONFIG.muted),
        ("2. fund_escrow — tokens move to pallet pot", CONFIG.payer),
        ("3. AwaitingRelease", CONFIG.pallet),
        ("4a. Resolvers vote; strict majority approve\n    → Released → payee paid", CONFIG.resolver),
        ("4b. Strict majority reject\n    → Refunded → payer", CONFIG.payee),
        ("4c. No majority and block > deadline\n    → payer calls refund_after_deadline → Refunded", CONFIG.payer),
        ("5. Terminal: Released or Refunded\n    (no further votes or timeout reclaim)", CONFIG.muted),
    ]
    w, h = 7.0, 0.78
    x0 = 1.0
    y = 10.85
    gap = 1.38
    prev: dict | None = None
    for i, (s, col) in enumerate(steps):
        b = _rounded_box(ax, (x0, y), w, h, s, col, fontsize=8.5)
        if prev is not None:
            _arrow_v(
                ax,
                b["cx"],
                prev["bot"] - 0.06,
                b["top"] + 0.06,
                color=CONFIG.edge_neutral,
                lw=1.3,
            )
        prev = b
        y -= gap

    ax.text(
        4.5,
        0.75,
        "Quorum: strict majority of configured resolvers (e.g. 2 of 3, 3 of 5).\n"
        "If n = 1 resolver, the first vote settles the escrow.",
        ha="center",
        fontsize=8,
        color=CONFIG.muted,
    )
    return fig


def fig_decision_branches() -> plt.Figure:
    fig, ax = plt.subplots(figsize=(14, 10), facecolor=CONFIG.face)
    ax.set_facecolor(CONFIG.face)
    ax.set_xlim(0, 14)
    ax.set_ylim(0, 10)
    ax.axis("off")

    ax.text(
        7,
        9.45,
        "Konnex escrow — outcomes after fund_escrow",
        ha="center",
        fontsize=14,
        fontweight="bold",
        color=CONFIG.text,
    )

    b_wait = _rounded_box(ax, (5.0, 7.85), 4.0, 0.65, "AwaitingRelease\n(funds in pot)", CONFIG.pallet, fontsize=10)

    cx, cy = 7.0, 6.05
    d = 0.62
    poly = Polygon(
        [(cx, cy + d), (cx + d, cy), (cx, cy - d), (cx - d, cy)],
        closed=True,
        facecolor=CONFIG.resolver,
        edgecolor=CONFIG.muted,
        linewidth=1.2,
        alpha=CONFIG.box_alpha,
    )
    ax.add_patch(poly)
    ax.text(cx, cy, "Approve\nquorum?", ha="center", va="center", fontsize=9, color=CONFIG.text)

    _arrow_v(ax, cx, b_wait["bot"] - 0.02, cy + d + 0.02, color=CONFIG.edge_neutral, lw=1.3)

    b_yes = _rounded_box(ax, (0.9, 4.95), 2.8, 0.58, "Released\n→ payee", CONFIG.payee, fontsize=9)
    _arrow(
        ax,
        (cx - d - 0.02, cy + 0.05),
        (b_yes["cx"], b_yes["top"] + 0.05),
        color=CONFIG.edge_ok,
        text="yes",
        fs=8,
        label_dx=-0.45,
        label_dy=0.25,
    )

    b_mid = _rounded_box(ax, (9.8, 4.85), 3.6, 0.68, "No approve quorum yet\n(or reject quorum)", CONFIG.resolver, fontsize=8.5)
    _arrow(ax, (cx + d + 0.02, cy + 0.1), (b_mid["x"], b_mid["cy"]), color=CONFIG.edge_neutral, text="no", fs=8, label_dx=0.35, label_dy=0.18)

    b_rej = _rounded_box(ax, (10.2, 2.95), 3.2, 0.58, "Refunded\n(reject quorum)", CONFIG.payer, fontsize=9)
    _arrow_v(ax, b_mid["cx"], b_mid["bot"] - 0.02, b_rej["top"] + 0.02, color=CONFIG.edge_warn, text="reject quorum", fs=7.5, label_dy=-0.25)

    b_dead = _rounded_box(ax, (4.2, 2.95), 4.0, 0.68, "block > deadline\nand still no approve quorum?", CONFIG.payer, fontsize=8.5)
    _arrow(ax, (b_mid["x"] + 0.2, b_mid["bot"] - 0.05), (b_dead["cx"] + 1.2, b_dead["top"] + 0.02), color=CONFIG.edge_neutral, lw=1.2, text="no reject yet", fs=7, label_dx=0.0, label_dy=0.12)

    b_to = _rounded_box(ax, (5.0, 1.05), 3.6, 0.58, "Refunded\n(refund_after_deadline)", CONFIG.payer, fontsize=9)
    _arrow_v(ax, b_dead["cx"], b_dead["bot"] - 0.02, b_to["top"] + 0.02, color=CONFIG.edge_neutral, text="yes", fs=8, label_dy=0.0)

    ax.text(
        7,
        0.35,
        "Reject quorum refunds immediately; otherwise payer may reclaim after deadline if approve quorum never forms.",
        ha="center",
        fontsize=8,
        color=CONFIG.muted,
    )
    return fig


def main() -> None:
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    makers = [
        ("konnex_escrow_1_swimlane.png", fig_swimlane),
        ("konnex_escrow_2_state_machine.png", fig_state_machine),
        ("konnex_escrow_3_linear_flow.png", fig_compact_flow),
        ("konnex_escrow_4_decision_tree.png", fig_decision_branches),
    ]
    for name, fn in makers:
        fig = fn()
        path = OUT_DIR / name
        fig.savefig(
            path,
            dpi=CONFIG.dpi,
            facecolor=CONFIG.face,
            bbox_inches="tight",
            pad_inches=0.35,
        )
        plt.close(fig)
        print("Wrote", path)


if __name__ == "__main__":
    main()
