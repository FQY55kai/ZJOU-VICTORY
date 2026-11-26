#!/usr/bin/env python3
"""Compare two frametime dumps from `VICTORY_FRAMETIME_DUMP`.

Why this exists: on one machine, one build, one switch set, the frame time of
this title swings by about 9% between runs, because the camera goes elsewhere.
A single run therefore settles nothing. This script reports, in the order that
matters:

  1. the counters, which repeat to about 5% (bytes copied, ranges sent, passes)
  2. the frame time, next to that noise band, with the ratio spelled out
  3. a pairing of frames by scene fingerprint (draws, copies, textures, ranges),
     so that "same work, different time" can be looked at directly

    python tools/bench/compare.py a.csv b.csv --label A --label B [--json out.json]

Rows are the in-world ones by default (`draws > 1000`); `--all` keeps the
loading phase too.
"""

import argparse
import json
import statistics as st
import sys

# The columns worth comparing, and whether a fall means less work.
COUNTERS = [
    "copies_mb", "textures_mb", "copyback_mb", "card_sent_mb",
    "card_fetched_mb", "card_ranges", "card_pieces", "heap_passes", "draws", "dropped",
]
TIMING = ["ms_per_frame", "cpu_prepare_ms", "copies_ms", "textures_ms",
          "copyback_ms", "heap_ms", "gpu_ms"]
FINGERPRINT = [("draws", 1000.0), ("copies_mb", 40.0), ("textures_mb", 15.0), ("card_ranges", 10000.0)]


def load(path, in_world=True):
    """The dump's data rows as dicts of floats; `#` lines are the header block."""
    rows = []
    with open(path, encoding="utf-8", errors="replace") as f:
        header = None
        for line in f:
            line = line.rstrip("\n")
            if not line:
                continue
            if line.startswith("#"):
                continue
            if header is None:
                header = line.split(",")
                continue
            rows.append({k: float(v) for k, v in zip(header, line.split(","))})
    if not rows:
        return [], None
    t0 = rows[0]["t_ms"]
    for r in rows:
        r["t_s"] = (r["t_ms"] - t0) / 1000.0
    if in_world:
        rows = [r for r in rows if r.get("draws", 0) > 1000]
    return rows, header


def med(rows, key):
    values = [r[key] for r in rows if key in r]
    return st.median(values) if values else float("nan")


def quantiles(rows, key, lo=0, hi=8, n=10):
    values = sorted(r[key] for r in rows if key in r)
    if len(values) < n:
        return (float("nan"), float("nan"))
    q = st.quantiles(values, n=n)
    return (q[lo], q[hi])


def fingerprint(row):
    return [row.get(k, 0.0) / scale for k, scale in FINGERPRINT]


def distance(a, b):
    return sum((x - y) ** 2 for x, y in zip(a, b)) ** 0.5


def pair_by_scene(a, b, limit):
    """Each frame of `b` against the nearest frame of `a` in fingerprint space."""
    pool = [(fingerprint(r), r["ms_per_frame"]) for r in a]
    pairs = []
    for r in b:
        fp = fingerprint(r)
        best = min(pool, key=lambda p: distance(p[0], fp))
        d = distance(best[0], fp)
        if d <= limit:
            pairs.append((d, best[1], r["ms_per_frame"]))
    return pairs


def change_cell(va, vb):
    """`+12.3%`, or `n/a` when the baseline is zero (GPU 0.0 ms is normal here)."""
    if not va:
        return "      n/a"
    return f"{(vb / va - 1) * 100:+9.1f}%"


def report(a, b, label_a, label_b, limit):
    out = {"pairing_limit": limit, "frames": {label_a: len(a), label_b: len(b)}}

    print(f"frames in world: {label_a} {len(a)}, {label_b} {len(b)}")
    print(f"\n{'counter':18s}{label_a:>12s}{label_b:>12s}{'change':>10s}")
    counters = {}
    for key in COUNTERS:
        va, vb = med(a, key), med(b, key)
        change = (vb / va - 1) * 100 if va else None
        counters[key] = {"a": va, "b": vb, "change_pct": change}
        print(f"{key:18s}{va:12.1f}{vb:12.1f}{change_cell(va, vb)}")
    out["counters"] = counters

    print(f"\n{'timing':18s}{label_a:>12s}{label_b:>12s}{'change':>10s}")
    timing = {}
    for key in TIMING:
        va, vb = med(a, key), med(b, key)
        change = (vb / va - 1) * 100 if va else None
        timing[key] = {"a": va, "b": vb, "change_pct": change}
        print(f"{key:18s}{va:12.1f}{vb:12.1f}{change_cell(va, vb)}")
    out["timing"] = timing

    lo_a, hi_a = quantiles(a, "ms_per_frame")
    lo_b, hi_b = quantiles(b, "ms_per_frame")
    print(f"\nms_per_frame p10-p90: {label_a} [{lo_a:.1f}, {hi_a:.1f}]   {label_b} [{lo_b:.1f}, {hi_b:.1f}]")
    out["p10_p90"] = {label_a: [lo_a, hi_a], label_b: [lo_b, hi_b]}

    pairs = pair_by_scene(a, b, limit)
    print(f"\npairing by scene fingerprint (distance <= {limit}): {len(pairs)} of {len(b)} frames")
    if len(pairs) >= 20:
        diffs = [y - x for _, x, y in pairs]
        print(f"  {label_a} paired median {st.median([p[1] for p in pairs]):.1f} ms, "
              f"{label_b} median {st.median([p[2] for p in pairs]):.1f} ms")
        print(f"  per-pair difference: median {st.median(diffs):+.1f} ms, mean {st.mean(diffs):+.1f} ms, "
              f"{sum(1 for d in diffs if d < 0) / len(diffs) * 100:.0f}% faster")
        out["pairing"] = {
            "pairs": len(pairs),
            "median_a": st.median([p[1] for p in pairs]),
            "median_b": st.median([p[2] for p in pairs]),
            "median_diff": st.median(diffs),
            "faster_share": sum(1 for d in diffs if d < 0) / len(diffs),
        }
    else:
        print("  too few pairs to say anything; the two runs went to different places")
        out["pairing"] = {"pairs": len(pairs)}
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("csv", nargs="+", help="two or more frametime dumps")
    ap.add_argument("--label", action="append", default=[], help="name for each dump, in order")
    ap.add_argument("--all", action="store_true", help="keep the loading phase too (draws > 0)")
    ap.add_argument("--limit", type=float, default=0.30, help="fingerprint distance for a pair (default 0.30)")
    ap.add_argument("--json", help="write the numbers here as well")
    args = ap.parse_args()

    labels = args.label + [f"run{i + 1}" for i in range(len(args.csv))][len(args.label):]
    loaded = [load(path, in_world=not args.all)[0] for path in args.csv]
    for path, rows in zip(args.csv, loaded):
        if not rows:
            sys.exit(f"{path}: no rows (wrong path, or every frame was filtered out)")
    if len(loaded) == 2:
        out = report(loaded[0], loaded[1], labels[0], labels[1], args.limit)
    else:
        out = {"runs": {}}
        for path, rows, label in zip(args.csv, loaded, labels):
            out["runs"][label] = {key: med(rows, key) for key in COUNTERS + TIMING}
            print(json.dumps({label: out["runs"][label]}, indent=2, ensure_ascii=False))
    if args.json:
        with open(args.json, "w", encoding="utf-8") as f:
            json.dump(out, f, indent=2, ensure_ascii=False)
        print(f"\nwritten {args.json}")


if __name__ == "__main__":
    main()
