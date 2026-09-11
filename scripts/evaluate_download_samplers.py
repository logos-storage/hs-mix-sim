#!/usr/bin/env python3
"""Run Rust anonymous-download evaluations and plot simulated vs formula S-DLM."""

from __future__ import annotations

import argparse
import csv
import math
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

import matplotlib

if "--show" not in sys.argv:
    matplotlib.use("Agg")
import matplotlib.pyplot as plt


MIB = 1024 * 1024
SIZE_EXPONENTS = tuple(range(11))


@dataclass(frozen=True)
class Variant:
    strategy: str
    key: str
    label: str
    rust_args: tuple[str, ...]


@dataclass(frozen=True)
class Result:
    variant: Variant
    exponent: int
    size_mib: int
    size_bytes: int
    session_paths: int
    users: int
    compromised_users: int
    simulated_probability: float
    confidence_low: float
    confidence_high: float
    formula_beta: float
    formula_probability: float


VARIANTS = (
    Variant("random", "random", "Uniform random", ("--mode", "random")),
    Variant(
        "k_hf",
        "k_hf_1",
        "K-HF ($h_f=1$)",
        ("--mode", "k-hf", "--fixed-hops", "1"),
    ),
    Variant(
        "k_hf",
        "k_hf_2",
        "K-HF ($h_f=2$)",
        ("--mode", "k-hf", "--fixed-hops", "2"),
    ),
    Variant(
        "k_over_w",
        "k_over_w_5",
        "K/W ($K=5$)",
        ("--mode", "k-w", "--k", "5"),
    ),
    Variant(
        "alpha_ss",
        "alpha_ss_095",
        "Alpha-SS ($\\alpha=0.95$)",
        ("--mode", "alpha-sticky", "--alpha", "0.95"),
    ),
    Variant(
        "alpha_ss",
        "alpha_ss_098",
        "Alpha-SS ($\\alpha=0.98$)",
        ("--mode", "alpha-sticky", "--alpha", "0.98"),
    ),
)

STRATEGY_LABELS = {
    "random": "Uniform random",
    "k_hf": "K-HF",
    "k_over_w": "K/W",
    "alpha_ss": "Alpha-SS",
}


def crate_directory() -> Path:
    return Path(__file__).resolve().parents[1]


def default_binary() -> Path:
    return crate_directory().parents[1] / "target" / "release" / "freeroutesim"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Run freeroutesim for 1-1024 MiB downloads and plot simulated and "
            "formula S-DLM."
        )
    )
    parser.add_argument(
        "--binary",
        type=Path,
        default=default_binary(),
        help="Path to the release freeroutesim binary",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=Path("results/download_sampler_evaluation"),
    )
    parser.add_argument("--users", type=int, default=5000)
    parser.add_argument("--hops", type=int, default=3)
    parser.add_argument("--packet-size", type=int, default=4608)
    parser.add_argument(
        "--force",
        action="store_true",
        help="Ignore cached raw summaries and rerun every Rust command",
    )
    parser.add_argument("--show", action="store_true")
    args = parser.parse_args()

    if args.users <= 0:
        parser.error("--users must be greater than zero")
    if args.hops <= 0:
        parser.error("--hops must be greater than zero")
    if args.packet_size <= 0:
        parser.error("--packet-size must be greater than zero")
    args.binary = args.binary.expanduser().resolve()
    args.output_dir = args.output_dir.expanduser().resolve()
    if not args.binary.is_file():
        parser.error(
            f"release binary not found at {args.binary}; run `cargo build --release` first"
        )
    return args


def parse_summary(output: str) -> dict[str, str]:
    summary: dict[str, str] = {}
    in_summary = False
    for raw_line in output.splitlines():
        line = raw_line.strip()
        if line == "simulation_summary":
            in_summary = True
            continue
        if in_summary and "=" in line:
            key, value = line.split("=", 1)
            summary[key] = value
    required = {
        "users",
        "session_paths",
        "formula_beta",
        "approximate_s_dlm_probability",
        "users_with_compromised_messages",
        "percentage_users_compromised",
    }
    missing = required.difference(summary)
    if missing:
        raise ValueError(f"Rust summary is missing: {', '.join(sorted(missing))}")
    return summary


def raw_result_path(
    output_dir: Path,
    variant: Variant,
    size_mib: int,
    users: int,
    hops: int,
    packet_size: int,
) -> Path:
    return output_dir / "raw" / (
        f"{variant.key}_{size_mib}mib_u{users}_h{hops}_p{packet_size}.txt"
    )


def command_for(
    binary: Path,
    variant: Variant,
    size_bytes: int,
    users: int,
    hops: int,
    packet_size: int,
) -> list[str]:
    return [
        str(binary),
        *variant.rust_args,
        "--model",
        "download-session",
        "--hops",
        str(hops),
        "--file-size",
        str(size_bytes),
        "--packet-size",
        str(packet_size),
        "--users",
        str(users),
    ]


def run_or_read_summary(
    args: argparse.Namespace,
    variant: Variant,
    size_mib: int,
    progress: str,
) -> dict[str, str]:
    raw_path = raw_result_path(
        args.output_dir,
        variant,
        size_mib,
        args.users,
        args.hops,
        args.packet_size,
    )
    if raw_path.exists() and not args.force:
        print(f"{progress} cached {variant.key}, {size_mib} MiB", flush=True)
        return parse_summary(raw_path.read_text(encoding="utf-8"))

    size_bytes = size_mib * MIB
    command = command_for(
        args.binary,
        variant,
        size_bytes,
        args.users,
        args.hops,
        args.packet_size,
    )
    print(f"{progress} running {variant.key}, {size_mib} MiB", flush=True)
    completed = subprocess.run(
        command,
        cwd=crate_directory(),
        capture_output=True,
        text=True,
        check=False,
    )
    if completed.returncode != 0:
        raise RuntimeError(
            f"command failed ({completed.returncode}): {' '.join(command)}\n"
            f"stdout:\n{completed.stdout}\nstderr:\n{completed.stderr}"
        )
    summary = parse_summary(completed.stdout)
    if int(summary["users"]) != args.users:
        raise ValueError("Rust summary reported an unexpected user count")
    raw_path.parent.mkdir(parents=True, exist_ok=True)
    raw_path.write_text(completed.stdout, encoding="utf-8")
    return summary


def wilson_interval(successes: int, trials: int) -> tuple[float, float]:
    z = 1.959963984540054
    estimate = successes / trials
    denominator = 1.0 + z * z / trials
    center = (estimate + z * z / (2.0 * trials)) / denominator
    radius = (
        z
        * math.sqrt(
            estimate * (1.0 - estimate) / trials
            + z * z / (4.0 * trials * trials)
        )
        / denominator
    )
    return max(0.0, center - radius), min(1.0, center + radius)


def collect_results(args: argparse.Namespace) -> list[Result]:
    total = len(VARIANTS) * len(SIZE_EXPONENTS)
    current = 0
    results: list[Result] = []
    for variant in VARIANTS:
        for exponent in SIZE_EXPONENTS:
            current += 1
            size_mib = 2**exponent
            summary = run_or_read_summary(
                args,
                variant,
                size_mib,
                f"[{current:02d}/{total}]",
            )
            users = int(summary["users"])
            compromised = int(summary["users_with_compromised_messages"])
            simulated = float(summary["percentage_users_compromised"]) / 100.0
            confidence_low, confidence_high = wilson_interval(compromised, users)
            results.append(
                Result(
                    variant=variant,
                    exponent=exponent,
                    size_mib=size_mib,
                    size_bytes=size_mib * MIB,
                    session_paths=int(summary["session_paths"]),
                    users=users,
                    compromised_users=compromised,
                    simulated_probability=simulated,
                    confidence_low=confidence_low,
                    confidence_high=confidence_high,
                    formula_beta=float(summary["formula_beta"]),
                    formula_probability=float(
                        summary["approximate_s_dlm_probability"]
                    ),
                )
            )
    return results


def write_csv(results: list[Result], path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="", encoding="utf-8") as csv_file:
        writer = csv.writer(csv_file)
        writer.writerow(
            [
                "strategy",
                "variant",
                "download_exponent",
                "download_size_mib",
                "download_size_bytes",
                "session_paths",
                "users",
                "compromised_users",
                "simulated_s_dlm",
                "simulated_ci95_low",
                "simulated_ci95_high",
                "formula_beta",
                "formula_s_dlm",
                "absolute_difference",
            ]
        )
        for result in results:
            writer.writerow(
                [
                    result.variant.strategy,
                    result.variant.key,
                    result.exponent,
                    result.size_mib,
                    result.size_bytes,
                    result.session_paths,
                    result.users,
                    result.compromised_users,
                    f"{result.simulated_probability:.10f}",
                    f"{result.confidence_low:.10f}",
                    f"{result.confidence_high:.10f}",
                    f"{result.formula_beta:.8f}",
                    f"{result.formula_probability:.10f}",
                    f"{abs(result.simulated_probability - result.formula_probability):.10f}",
                ]
            )


def plot_strategy(
    strategy: str,
    results: list[Result],
    output: Path,
    show: bool,
) -> None:
    variants = [variant for variant in VARIANTS if variant.strategy == strategy]
    colors = plt.get_cmap("tab10").colors
    figure, axis = plt.subplots(figsize=(9.5, 5.8))

    for variant, color in zip(variants, colors):
        selected = [result for result in results if result.variant == variant]
        sizes = [result.size_mib for result in selected]
        variant_suffix = "" if len(variants) == 1 else f" — {variant.label}"
        axis.plot(
            sizes,
            [result.simulated_probability for result in selected],
            marker="o",
            linewidth=2,
            color=color,
            label=f"Simulated{variant_suffix}",
        )
        axis.plot(
            sizes,
            [result.formula_probability for result in selected],
            marker="s",
            linestyle="--",
            linewidth=2,
            color=color,
            label=f"Formula{variant_suffix}",
        )

    sizes = [2**exponent for exponent in SIZE_EXPONENTS]
    axis.set_xscale("log", base=2)
    axis.set_xticks(sizes, [str(size) for size in sizes])
    axis.set_xlim(sizes[0] / math.sqrt(2), sizes[-1] * math.sqrt(2))
    axis.set_ylim(0.0, 1.01)
    axis.set_xlabel("Download size (MiB)")
    axis.set_ylabel("S-DLM")
    axis.set_title(f"Anonymous download S-DLM — {STRATEGY_LABELS[strategy]}")
    axis.grid(True, which="both", alpha=0.3)
    axis.legend()
    figure.tight_layout()
    output.parent.mkdir(parents=True, exist_ok=True)
    figure.savefig(output, dpi=200, bbox_inches="tight")
    print(f"wrote {output}")
    if show:
        plt.show()
    else:
        plt.close(figure)


def probability(value: float) -> str:
    return f"{value:.6f}"


def write_markdown(results: list[Result], args: argparse.Namespace) -> Path:
    output = args.output_dir / "EVALUATION.md"
    lines = [
        "# Anonymous download sampler evaluation",
        "",
        "## Configuration",
        "",
        "| Parameter | Value |",
        "| --- | ---: |",
        f"| Rust simulations per point | {args.users:,} users |",
        f"| Path hops $L$ | {args.hops} |",
        f"| Total Sphinx packet size | {args.packet_size} bytes |",
        "| K-HF fixed-hop variants | $h_f=1$, $h_f=2$ |",
        "| K/W candidates | $K=5$ |",
        "| Alpha-SS variants | $\\alpha=0.95$, $\\alpha=0.98$ |",
        "| File sizes | $2^0$ through $2^{10}$ MiB |",
        "",
        "## Method",
        "",
        "Every simulated point was produced by the release Rust `freeroutesim` binary "
        "using the implemented anonymous-download model and path sampler. The Python "
        "script only runs the commands, parses their summaries, and creates this report "
        "and the plots. It does not simulate sampler behavior.",
        "",
        "Each Rust invocation generates a fresh mixnet. Consequently, the realized "
        "malicious fraction can differ between points. The formula curve uses the "
        "`formula_beta` calculated from that invocation's actual mixnet, keeping each "
        "formula point paired with its simulated point. The simulator currently has no "
        "seed option, so rerunning with `--force` produces a new Monte Carlo sample.",
        "",
        "The formula values are the approximations selected in "
        "[`MIX_PATH_SELECTION.md`](../../MIX_PATH_SELECTION.md). The K/W formula is the "
        "document's upper-bound approximation. The plots omit confidence bands so the "
        "single-parameter strategies have exactly two curves; 95% Wilson intervals are "
        "included in the tables and CSV.",
        "",
        "Raw results are in [`evaluation.csv`](evaluation.csv), and the original Rust "
        "summaries are retained under [`raw/`](raw/).",
        "",
        "## Endpoint summary",
        "",
        "| Variant | Simulated at 1 MiB | Formula at 1 MiB | Simulated at 1024 MiB | Formula at 1024 MiB | Maximum absolute difference |",
        "| --- | ---: | ---: | ---: | ---: | ---: |",
    ]

    for variant in VARIANTS:
        selected = [result for result in results if result.variant == variant]
        maximum_difference = max(
            abs(result.simulated_probability - result.formula_probability)
            for result in selected
        )
        lines.append(
            f"| {variant.label} | {probability(selected[0].simulated_probability)} | "
            f"{probability(selected[0].formula_probability)} | "
            f"{probability(selected[-1].simulated_probability)} | "
            f"{probability(selected[-1].formula_probability)} | "
            f"{probability(maximum_difference)} |"
        )

    for strategy in STRATEGY_LABELS:
        variants = [variant for variant in VARIANTS if variant.strategy == strategy]
        lines.extend(
            [
                "",
                f"## {STRATEGY_LABELS[strategy]}",
                "",
                f"![{STRATEGY_LABELS[strategy]} S-DLM](./{strategy}.png)",
                "",
                "| Variant | Download (MiB) | $N_{session}$ | $\\beta$ | Simulated S-DLM | 95% CI | Formula S-DLM | Absolute difference |",
                "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
            ]
        )
        for variant in variants:
            selected = [result for result in results if result.variant == variant]
            for result in selected:
                lines.append(
                    f"| {variant.label} | {result.size_mib} | "
                    f"{result.session_paths:,} | {result.formula_beta:.4f} | "
                    f"{probability(result.simulated_probability)} | "
                    f"[{probability(result.confidence_low)}, "
                    f"{probability(result.confidence_high)}] | "
                    f"{probability(result.formula_probability)} | "
                    f"{probability(abs(result.simulated_probability - result.formula_probability))} |"
                )

    lines.extend(
        [
            "",
            "## Interpretation notes",
            "",
            "- Uniform random rapidly approaches S-DLM 1 as independent path exposure accumulates.",
            "- K-HF with two fixed hops has a lower large-session ceiling than K-HF with one fixed hop: approximately $\\beta^2$ rather than $\\beta$.",
            "- K/W uses five candidates per hop. Its document formula remains an upper bound and can be substantially above the implemented globally disjoint-pool simulation.",
            "- Alpha-SS with $\\alpha=0.98$ introduces fewer distinct paths than $\\alpha=0.95$, delaying compromise as download size grows.",
            "- Point-to-point variation includes both finite-user Monte Carlo error and fresh-mixnet variation.",
            "",
            "## Reproduction",
            "",
            "From the `freeroutesim` crate directory:",
            "",
            "```bash",
            "cargo build --release",
            "python3 scripts/evaluate_download_samplers.py --force",
            "```",
            "",
        ]
    )
    output.write_text("\n".join(lines), encoding="utf-8")
    print(f"wrote {output}")
    return output


def main() -> None:
    args = parse_args()
    results = collect_results(args)
    args.output_dir.mkdir(parents=True, exist_ok=True)
    write_csv(results, args.output_dir / "evaluation.csv")
    for strategy in STRATEGY_LABELS:
        plot_strategy(
            strategy,
            results,
            args.output_dir / f"{strategy}.png",
            args.show,
        )
    write_markdown(results, args)


if __name__ == "__main__":
    main()
