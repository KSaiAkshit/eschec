import re
import sys
from pathlib import Path
from collections import defaultdict


def parse_log_file(log_path):
    """Parse a single log file for cutoff stats."""
    stats = {
        "total_nodes": 0,
        "cutoff_nodes": 0,
        "cutoff_at": defaultdict(int),
        "searches": 0,
    }

    with open(log_path, "r") as f:
        for line in f:
            # Parse summary line
            match = re.search(
                r"CUTOFF_STATS depth=(\d+) total_nodes=(\d+) cutoff_nodes=(\d+) "
                r"cutoff_rate=([\d.]+) avg_cutoff_at=([\d.]+)",
                line,
            )
            if match:
                stats["searches"] += 1
                stats["total_nodes"] += int(match.group(2))
                stats["cutoff_nodes"] += int(match.group(3))

            # Parse histogram line
            match = re.search(r"CUTOFF_HISTOGRAM.*data=\[(.*?)\]", line)
            if match:
                histogram_data = match.group(1)
                for entry in histogram_data.split(","):
                    if ":" in entry:
                        idx, count = entry.split(":")
                        stats["cutoff_at"][int(idx)] += int(count)

    return stats


def aggregate_stats(log_dir):
    """Aggregate statistics from all log files in directory."""
    log_files = list(Path(log_dir).glob("eschec_*.log"))

    if not log_files:
        print(f"No log files found in {log_dir}")
        return None

    print(f"Found {len(log_files)} log files")

    total_stats = {
        "total_nodes": 0,
        "cutoff_nodes": 0,
        "cutoff_at": defaultdict(int),
        "searches": 0,
    }

    for log_file in log_files:
        stats = parse_log_file(log_file)
        total_stats["total_nodes"] += stats["total_nodes"]
        total_stats["cutoff_nodes"] += stats["cutoff_nodes"]
        total_stats["searches"] += stats["searches"]
        for idx, count in stats["cutoff_at"].items():
            total_stats["cutoff_at"][idx] += count

    return total_stats


def print_analysis(stats):
    """Print analysis of aggregated statistics."""
    if not stats or stats["total_nodes"] == 0:
        print("No statistics to analyze")
        return

    print("\n" + "=" * 70)
    print("CUTOFF ANALYSIS")
    print("=" * 70)

    print(f"\nTotal searches:  {stats['searches']:>12,}")
    print(f"Total nodes:     {stats['total_nodes']:>12,}")
    print(f"Cutoff nodes:    {stats['cutoff_nodes']:>12,}")

    cutoff_rate = 100.0 * stats["cutoff_nodes"] / stats["total_nodes"]
    print(f"Cutoff rate:     {cutoff_rate:>11.2f}%")

    # Average cutoff position
    total_weighted = sum(idx * count for idx, count in stats["cutoff_at"].items())
    avg_cutoff = (
        total_weighted / stats["cutoff_nodes"] if stats["cutoff_nodes"] > 0 else 0
    )
    print(f"Avg cutoff at:   {avg_cutoff:>11.2f} (move index)")

    print("\nCutoff Distribution:")
    print(f"  {'Move':<6} {'Count':>12} {'Percent':>10} {'Cumulative':>12}")
    print(f"  {'-' * 6} {'-' * 12} {'-' * 10} {'-' * 12}")

    cumulative = 0
    for idx in sorted(stats["cutoff_at"].keys()):
        count = stats["cutoff_at"][idx]
        cumulative += count
        pct = 100.0 * count / stats["cutoff_nodes"]
        cum_pct = 100.0 * cumulative / stats["cutoff_nodes"]
        print(f"  {idx:>4}   {count:>12,} {pct:>9.2f}% {cum_pct:>11.2f}%")

        if cum_pct > 99.0:
            break

    # Key insights
    print("\n" + "=" * 70)
    print("KEY INSIGHTS")
    print("=" * 70)

    first_move_pct = 100.0 * stats["cutoff_at"].get(0, 0) / stats["cutoff_nodes"]
    print(f"\nFirst move cutoffs: {first_move_pct:.1f}%")

    top_3 = sum(stats["cutoff_at"].get(i, 0) for i in range(3))
    top_3_pct = 100.0 * top_3 / stats["cutoff_nodes"]
    print(f"Top 3 moves:        {top_3_pct:.1f}%")

    top_5 = sum(stats["cutoff_at"].get(i, 0) for i in range(5))
    top_5_pct = 100.0 * top_5 / stats["cutoff_nodes"]
    print(f"Top 5 moves:        {top_5_pct:.1f}%")

    print("\n" + "=" * 70 + "\n")


if __name__ == "__main__":
    log_dir = sys.argv[1] if len(sys.argv) > 1 else "/tmp/eschec_logs"
    stats = aggregate_stats(log_dir)
    if stats:
        print_analysis(stats)
