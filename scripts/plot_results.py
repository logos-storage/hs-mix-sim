#!/usr/bin/env python3
"""Plot freeroutesim's model-specific CSVs; all labels come from run metadata."""

import argparse
import csv
import hashlib
import math
from pathlib import Path
import textwrap

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.ticker import PercentFormatter

MODELS = {"SimpleModel", "HiddenServiceModel", "DownloadSessionModel"}
DATA_FIELDS = {
    "curve", "x", "compromised_users", "cumulative_probability",
    "mean_node_compromises_before_win", "download_size_bytes", "packet_count",
    "simulated_s_dlm", "formula_s_dlm",
}
COMMON_FIELDS = {"model", "sampler", "hops", "adversary", "users", "mix_nodes",
                 "malicious_nodes", "malicious_node_fraction"}


def read_result(path):
    with path.open(newline="") as stream:
        reader = csv.DictReader(stream)
        headers = set(reader.fieldnames or [])
        rows = list(reader)
    if not COMMON_FIELDS <= headers or not rows:
        raise ValueError(f"{path}: expected a new model-specific CSV with run metadata")
    model = rows[0]["model"]
    if model not in MODELS:
        raise ValueError(f"{path}: unsupported model {model!r}")
    download = model == "DownloadSessionModel"
    required = ({"download_size_bytes", "packet_count", "simulated_s_dlm", "formula_s_dlm"}
                if download else {"curve", "x", "cumulative_probability", "duration_seconds"})
    if not required <= headers:
        raise ValueError(f"{path}: missing columns {sorted(required - headers)}")
    metadata = {key: rows[0][key] for key in sorted(headers - DATA_FIELDS)}
    for row in rows:
        if None in row or any(value is None for value in row.values()):
            raise ValueError(f"{path}: malformed CSV row")
        if any(row[key] != value for key, value in metadata.items()):
            raise ValueError(f"{path}: configuration changes within one CSV")
        for key in (("simulated_s_dlm", "formula_s_dlm") if download else ("cumulative_probability",)):
            value = float(row[key])
            if not math.isfinite(value) or not 0 <= value <= 1:
                raise ValueError(f"{path}: invalid probability in {key}")
        if download:
            if int(row["download_size_bytes"]) <= 0 or int(row["packet_count"]) <= 0:
                raise ValueError(f"{path}: download size and packet count must be positive")
        elif row["curve"] not in {"time_seconds", "message_count"} or int(row["x"]) < 0:
            raise ValueError(f"{path}: invalid curve or coordinate")
    if not download:
        expected = {"time_seconds", "message_count"} if model == "SimpleModel" else {"time_seconds"}
        if {row["curve"] for row in rows} != expected:
            raise ValueError(f"{path}: expected curves {sorted(expected)}")
    return metadata, rows


def caption(metadata):
    parts = [f"{metadata['sampler']} | {metadata['hops']} hops | {metadata['adversary']}",
             f"{int(metadata['users']):,} runs; {metadata['mix_nodes']} mixes; "
             f"{metadata['malicious_nodes']} malicious ({float(metadata['malicious_node_fraction']):.1%})"]
    if "duration_seconds" in metadata:
        parts[-1] += f"; deadline {int(metadata['duration_seconds']) / 86400:g} days"
    details = []
    for key in ("fixed_hops", "k", "alpha", "packet_size_bytes"):
        if key in metadata:
            details.append(f"{key.replace('_', ' ')}={metadata[key]}")
    if "stored_path_count" in metadata:
        details.append(f"{metadata['stored_path_count']} stored paths; lifetime=max(two uniform draws, "
                       f"{int(metadata['path_lifetime_min_seconds']) / 3600:g}–"
                       f"{int(metadata['path_lifetime_max_seconds']) / 3600:g} h)")
    if "node_compromise_probability" in metadata:
        chance = float(metadata["node_compromise_probability"])
        details.append(f"per-node compromise: {chance:.0%}" +
                       (f"; delay uniform 1 s–{int(metadata['node_compromise_window_seconds']) / 86400:g} days, "
                        "otherwise never" if chance else " (Sybil control only)"))
    if "message_interval_min_seconds" in metadata:
        details.append(f"message interval: uniform [{metadata['message_interval_min_seconds']}, "
                       f"{metadata['message_interval_max_exclusive_seconds']}) seconds")
    return "\n".join(textwrap.fill(part, width=112) for part in parts + details)


def save_plot(fig, ax, metadata, path):
    ax.yaxis.set_major_formatter(PercentFormatter(xmax=1))
    ax.set_ylim(0, min(1, ax.get_ylim()[1]))
    ax.grid(True, alpha=0.25)
    note = caption(metadata)
    # Reserve real space for variable-length configuration instead of clipping it.
    footer_height = 0.22 * len(note.splitlines()) + 0.25
    fig.set_size_inches(11, 5.0 + footer_height)
    fig.tight_layout(rect=(0, footer_height / fig.get_figheight(), 1, 1))
    fig.text(0.08, 0.025, note, fontsize=9, va="bottom")
    fig.savefig(path, dpi=160)
    plt.close(fig)
    print(path)


def plot_curves(metadata, rows, output_dir, stem):
    hidden = metadata["model"] == "HiddenServiceModel"
    for curve, suffix, xlabel in (("time_seconds", "time", "Time (days)"),
                                  ("message_count", "messages", "Messages per user")):
        points = sorted((int(row["x"]), float(row["cumulative_probability"]))
                        for row in rows if row["curve"] == curve)
        if not points:
            continue
        fig, ax = plt.subplots()
        divisor = 86400 if curve == "time_seconds" else 1
        ax.step([x / divisor for x, _ in points], [y for _, y in points], where="post",
                marker="o" if len(points) == 1 else None)
        ax.set_title("Hidden service: cumulative compromise" if hidden else "Simple model: cumulative compromise")
        ax.set_xlabel(xlabel)
        ax.set_ylabel("Cumulative probability of compromise")
        save_plot(fig, ax, metadata, output_dir / f"{stem}_{suffix}.png")


def format_size(value):
    for unit, scale in (("GiB", 1024**3), ("MiB", 1024**2), ("KiB", 1024)):
        if value >= scale:
            return f"{value / scale:g} {unit}"
    return f"{value} B"


def plot_download(metadata, rows, output_dir):
    rows = sorted(rows, key=lambda row: int(row["download_size_bytes"]))
    sizes = [int(row["download_size_bytes"]) for row in rows]
    if len(set(sizes)) != len(sizes):
        raise ValueError("Duplicate download sizes for the same configuration; provide one CSV per size")
    fig, ax = plt.subplots()
    for key, label, style in (("simulated_s_dlm", "Simulated", "o-"),
                              ("formula_s_dlm", "Formula (approx.)", "s--")):
        ax.plot(sizes, [float(row[key]) for row in rows], style, label=label)
    ax.set_xscale("log", base=2)
    ax.set_xticks(sizes, [f"{format_size(size)}\n[{int(row['packet_count']):,} packets]"
                        for size, row in zip(sizes, rows)], rotation=30 if len(sizes) > 6 else 0)
    ax.minorticks_off()
    ax.set_xlabel("Download size [total session packet count] — logarithmic size axis")
    ax.set_ylabel("S-DLM")
    ax.set_title("Anonymous download: simulated and formula S-DLM")
    ax.legend()
    key = hashlib.sha256(repr(sorted(metadata.items())).encode()).hexdigest()[:8]
    save_plot(fig, ax, metadata, output_dir / f"download_{metadata['sampler']}_h{metadata['hops']}_{key}.png")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("csv_files", nargs="+", type=Path)
    parser.add_argument("--output-dir", type=Path, default=Path("results/plots"))
    args = parser.parse_args()
    args.output_dir.mkdir(parents=True, exist_ok=True)
    downloads = {}
    try:
        # A path digest prevents identical CSV basenames from overwriting plots.
        for path in args.csv_files:
            metadata, rows = read_result(path)
            if metadata["model"] == "DownloadSessionModel":
                key = tuple(sorted(metadata.items()))
                downloads.setdefault(key, []).extend(rows)
            else:
                key = hashlib.sha256(str(path.resolve()).encode()).hexdigest()[:8]
                plot_curves(metadata, rows, args.output_dir, f"{path.stem}_{key}")
        for key, rows in downloads.items():
            plot_download(dict(key), rows, args.output_dir)
    except (OSError, ValueError, KeyError) as error:
        parser.exit(1, f"Error: {error}\n")


if __name__ == "__main__":
    main()
