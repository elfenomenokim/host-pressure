from pathlib import Path
import json, statistics
root = Path(__file__).resolve().parents[2]
bench = root / "docs/bench"
m5 = json.loads((bench / "m5-no-sample.json").read_text())
values = [statistics.mean(v["time_wall_clock"]["value"] for v in r["measurements"]) * 1000 for r in m5["results"]]
values += [v["mean_ms"] for v in json.loads((bench / "a1-no-sample.json").read_text())["runs"].values()]
values += [v["mean_ms"] for v in json.loads((bench / "windows-no-sample.json").read_text())["runs"]]
for suffix, title, ylabel in [("", "--no-sample · 3 warmups, 30 runs", "Mean per call (ms)"), (".ko", "--no-sample · 예열 3회, 측정 30회", "호출당 평균 (ms)")]:
    source = ('xychart-beta\n    title "' + title + '"\n'
              '    x-axis ["M5 Python", "M5 Rust", "A1 Python", "A1 Rust", "Windows Python", "Windows Rust"]\n'
              '    y-axis "' + ylabel + '" 0 --> 180\n'
              '    bar [' + ', '.join(f'{v:.3f}' for v in values) + ']\n')
    (root / "docs/diagrams" / ("no-sample" + suffix + ".mmd")).write_text(source)
