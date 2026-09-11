#!/usr/bin/env python3
"""Plot cumulative compromise curves produced by freeroutesim."""

from __future__ import annotations

import argparse
import csv
import sys
from dataclasses import dataclass
from pathlib import Path

import matplotlib

if "--show" not in sys.argv:
    matplotlib.use("Agg")
import matplotlib.pyplot as plt


@dataclass
class Curve:
    x: list[float]
    probability: list[float]


@dataclass
class SimulationResult:
    label: str
    time: Curve
    messages: Curve


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Plot cumulative probability of first compromise against virtual "
            "time and number of adversary checks."
        )
    )
    parser.add_argument(
        "csv_files",
        nargs="+",
        type=Path,
        help="One or more CSV files written by freeroutesim --csv",
    )
    parser.add_argument(
        "-o",
        "--output",
        type=Path,
        help="Output image path (default: <first-csv-stem>_plot.png)",
    )
    parser.add_argument(
        "--labels",
        nargs="+",
        help="Legend labels in the same order as the CSV files",
    )
    parser.add_argument("--title", help="Optional figure title")
    parser.add_argument(
        "--log-messages",
        action="store_true",
        help="Use a logarithmic x-axis for the message-count plot",
    )
    parser.add_argument(
        "--show",
        action="store_true",
        help="Open the plot after saving it",
    )
    args = parser.parse_args()

    if args.labels is not None and len(args.labels) != len(args.csv_files):
        parser.error("--labels must contain exactly one label per CSV file")
    return args


def read_result(path: Path, label: str) -> SimulationResult:
    time_points: list[tuple[float, float]] = []
    message_points: list[tuple[float, float]] = []

    with path.open(newline="", encoding="utf-8") as csv_file:
        reader = csv.DictReader(csv_file)
        required = {"curve", "x", "day", "cumulative_probability"}
        missing = required.difference(reader.fieldnames or [])
        if missing:
            missing_columns = ", ".join(sorted(missing))
            raise ValueError(f"{path}: missing CSV columns: {missing_columns}")

        for line_number, row in enumerate(reader, start=2):
            try:
                probability = float(row["cumulative_probability"])
                if row["curve"] == "time_seconds":
                    time_points.append((float(row["day"]), probability))
                elif row["curve"] in {"message_count", "check_count"}:
                    message_points.append((float(row["x"]), probability))
            except (TypeError, ValueError) as error:
                raise ValueError(
                    f"{path}:{line_number}: invalid numeric value"
                ) from error

    if not time_points:
        raise ValueError(f"{path}: contains no time_seconds curve")
    if not message_points:
        raise ValueError(f"{path}: contains no message_count or check_count curve")

    time_points.sort()
    message_points.sort()
    return SimulationResult(
        label=label,
        time=Curve(
            x=[point[0] for point in time_points],
            probability=[point[1] for point in time_points],
        ),
        messages=Curve(
            x=[point[0] for point in message_points],
            probability=[point[1] for point in message_points],
        ),
    )


def plot_results(
    results: list[SimulationResult],
    output: Path,
    title: str | None,
    log_messages: bool,
    show: bool,
) -> None:
    figure, (time_axis, message_axis) = plt.subplots(1, 2, figsize=(13, 5.5))

    colors = plt.get_cmap("tab20").colors if len(results) > 10 else [None] * len(results)
    for result, color in zip(results, colors):
        time_axis.step(
            result.time.x,
            result.time.probability,
            where="post",
            label=result.label,
            color=color,
        )
        message_axis.step(
            result.messages.x,
            result.messages.probability,
            where="post",
            label=result.label,
            color=color,
        )

    configure_axis(
        time_axis,
        xlabel="time (days)",
        title="Time to first compromise",
    )
    configure_axis(
        message_axis,
        xlabel="Adversary checks",
        title="Checks to first compromise",
    )
    if log_messages:
        message_axis.set_xscale("symlog", linthresh=1)
        message_axis.set_xlim(left=0)

    if len(results) > 6:
        handles, labels = time_axis.get_legend_handles_labels()
        figure.legend(
            handles,
            labels,
            loc="lower center",
            ncol=6,
            bbox_to_anchor=(0.5, -0.01),
        )
    elif len(results) > 1:
        time_axis.legend()
        message_axis.legend()
    if title:
        figure.suptitle(title)

    figure.tight_layout(rect=(0, 0.09, 1, 1) if len(results) > 6 else None)
    output.parent.mkdir(parents=True, exist_ok=True)
    figure.savefig(output, dpi=200, bbox_inches="tight")
    print(f"wrote {output}")

    if show:
        plt.show()
    else:
        plt.close(figure)


def configure_axis(axis: plt.Axes, xlabel: str, title: str) -> None:
    axis.set_title(title)
    axis.set_xlabel(xlabel)
    axis.set_ylabel("Cumulative probability of compromise")
    axis.set_ylim(0.0, 1.01)
    axis.grid(True, alpha=0.3)


def main() -> None:
    args = parse_args()
    labels = args.labels or [path.stem for path in args.csv_files]
    results = [
        read_result(path, label) for path, label in zip(args.csv_files, labels)
    ]
    output = args.output or args.csv_files[0].with_name(
        f"{args.csv_files[0].stem}_plot.png"
    )
    plot_results(results, output, args.title, args.log_messages, args.show)


if __name__ == "__main__":
    main()
