#!/usr/bin/env python3
import argparse
import calendar
import csv
import math
from dataclasses import dataclass, field
from datetime import datetime
from pathlib import Path
from typing import Optional


@dataclass
class UserStats:
    messages: int = 0
    bursts_seen: set[str] = field(default_factory=set)
    first_compromise_seconds: float = math.inf
    messages_until_compromise: float = math.inf
    bursts_until_compromise: float = math.inf


def parse_bool(value: str) -> bool:
    lowered = value.strip().lower()
    if lowered in {"true", "1", "yes"}:
        return True
    if lowered in {"false", "0", "no"}:
        return False
    raise ValueError(f"invalid boolean: {value}")


def parse_timestamp(date_part: str, time_part: str) -> int:
    dt = datetime.fromisoformat(f"{date_part} {time_part}")
    return calendar.timegm(dt.timetuple())


def parse_console_line(line: str, expected_format: str):
    parts = line.split()
    if expected_format == "simple":
        if len(parts) != 5:
            return None
        date_part, time_part, user, _path, compromised = parts
        return parse_timestamp(date_part, time_part), int(user), None, parse_bool(compromised)

    if expected_format == "hidden-service":
        if len(parts) != 6:
            return None
        date_part, time_part, user, burst_id, _path, compromised = parts
        return parse_timestamp(date_part, time_part), int(user), burst_id, parse_bool(compromised)

    raise ValueError(f"unsupported format: {expected_format}")


def process_console_output(path: Path, expected_format: str, users: Optional[int]):
    stats: dict[int, UserStats] = {}
    skipped = 0

    with path.open() as infile:
        for line in infile:
            line = line.strip()
            if not line:
                continue
            parsed = parse_console_line(line, expected_format)
            if parsed is None:
                skipped += 1
                continue

            timestamp, user, burst_id, compromised = parsed
            user_stats = stats.setdefault(user, UserStats())
            user_stats.messages += 1
            if burst_id is not None:
                user_stats.bursts_seen.add(burst_id)

            if compromised and math.isinf(user_stats.first_compromise_seconds):
                user_stats.first_compromise_seconds = timestamp
                user_stats.messages_until_compromise = user_stats.messages
                if burst_id is not None:
                    user_stats.bursts_until_compromise = len(user_stats.bursts_seen)

    if users is None:
        users = max(stats.keys(), default=-1) + 1

    for user in range(users):
        stats.setdefault(user, UserStats())

    return stats, skipped


def finite_values(values):
    return sorted(value for value in values if math.isfinite(value))


def choose_time_unit(values):
    finite = finite_values(values)
    if not finite:
        return 1, "seconds"
    max_value = max(finite)
    if max_value <= 7 * 24 * 60 * 60:
        return 60 * 60, "hours"
    if max_value <= 30 * 24 * 60 * 60:
        return 24 * 60 * 60, "days"
    if max_value <= 12 * 30 * 24 * 60 * 60:
        return 7 * 24 * 60 * 60, "weeks"
    return 30 * 24 * 60 * 60, "months"


def plot_cdf(values, total_users, xlabel, out_file):
    import matplotlib

    matplotlib.use("PDF")
    import matplotlib.pyplot as plt

    finite = finite_values(values)
    fig, ax = plt.subplots(figsize=(6.4, 3.8))
    if finite:
        xs = []
        ys = []
        last_y = 0.0
        for idx, value in enumerate(finite, start=1):
            y = idx / total_users
            xs.extend([value, value])
            ys.extend([last_y, y])
            last_y = y
        ax.plot(xs, ys, "-o", linewidth=2, markevery=max(1, len(xs) // 10))
        ax.set_xlim(left=0)
    else:
        ax.text(0.5, 0.5, "No compromised users", ha="center", va="center")
        ax.set_xlim(0, 1)

    ax.set_ylim(0, 1)
    ax.set_xlabel(xlabel)
    ax.set_ylabel("Cumulative probability")
    ax.grid(True)
    fig.tight_layout()
    fig.savefig(out_file)
    plt.close(fig)


def write_per_user_csv(stats, out_file):
    with out_file.open("w", newline="") as outfile:
        writer = csv.writer(outfile)
        writer.writerow(
            [
                "user",
                "messages",
                "bursts",
                "first_compromise_seconds",
                "messages_until_compromise",
                "bursts_until_compromise",
            ]
        )
        for user in sorted(stats):
            user_stats = stats[user]
            writer.writerow(
                [
                    user,
                    user_stats.messages,
                    len(user_stats.bursts_seen),
                    "inf"
                    if math.isinf(user_stats.first_compromise_seconds)
                    else int(user_stats.first_compromise_seconds),
                    "inf"
                    if math.isinf(user_stats.messages_until_compromise)
                    else int(user_stats.messages_until_compromise),
                    "inf"
                    if math.isinf(user_stats.bursts_until_compromise)
                    else int(user_stats.bursts_until_compromise),
                ]
            )


def write_summary(stats, skipped, out_file):
    users = len(stats)
    compromised_users = sum(
        1 for user_stats in stats.values() if math.isfinite(user_stats.first_compromise_seconds)
    )
    total_messages = sum(user_stats.messages for user_stats in stats.values())
    compromised_pct = compromised_users / users * 100 if users else 0

    with out_file.open("w") as outfile:
        outfile.write(f"users={users}\n")
        outfile.write(f"total_messages={total_messages}\n")
        outfile.write(f"compromised_users={compromised_users}\n")
        outfile.write(f"percentage_compromised_users={compromised_pct:.6f}\n")
        outfile.write(f"skipped_lines={skipped}\n")


def main():
    parser = argparse.ArgumentParser(
        description="Parse routesim-v2 --to-console output and plot CDFs."
    )
    parser.add_argument("--in-file", required=True, type=Path)
    parser.add_argument("--out-prefix", required=True, type=Path)
    parser.add_argument("--users", type=int)
    parser.add_argument(
        "--format",
        choices=["simple", "hidden-service"],
        required=True,
        help="Expected routesim-v2 console output format.",
    )
    parser.add_argument(
        "--plots",
        default=None,
        help="Comma-separated plots to generate: time,messages,bursts. Defaults by format.",
    )
    args = parser.parse_args()

    stats, skipped = process_console_output(args.in_file, args.format, args.users)
    args.out_prefix.parent.mkdir(parents=True, exist_ok=True)

    write_per_user_csv(stats, args.out_prefix.with_suffix(".per_user.csv"))
    write_summary(stats, skipped, args.out_prefix.with_suffix(".summary.txt"))

    plots = (
        args.plots.split(",")
        if args.plots
        else (["time", "messages", "bursts"] if args.format == "hidden-service" else ["time", "messages"])
    )

    if "time" in plots:
        raw_values = [user_stats.first_compromise_seconds for user_stats in stats.values()]
        divider, unit = choose_time_unit(raw_values)
        plot_cdf(
            [value / divider if math.isfinite(value) else value for value in raw_values],
            len(stats),
            f"time to first compromise [{unit}]",
            args.out_prefix.with_name(args.out_prefix.name + "_time_cdf.pdf"),
        )

    if "messages" in plots:
        plot_cdf(
            [user_stats.messages_until_compromise for user_stats in stats.values()],
            len(stats),
            "messages until first compromise",
            args.out_prefix.with_name(args.out_prefix.name + "_messages_cdf.pdf"),
        )

    if "bursts" in plots:
        plot_cdf(
            [user_stats.bursts_until_compromise for user_stats in stats.values()],
            len(stats),
            "bursts until first compromise",
            args.out_prefix.with_name(args.out_prefix.name + "_bursts_cdf.pdf"),
        )


if __name__ == "__main__":
    main()
