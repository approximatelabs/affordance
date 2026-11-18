"""Compare bench/results_hf.json (Python: Jinja2 + tokenize) with bench/results_rust_1core.json
(view renderer) on the same cases. Writes bench/compare.md and bench/speed.png."""
import json, os, statistics
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt

R = os.path.dirname(os.path.abspath(__file__))
hf = json.load(open(f"{R}/results_hf.json"))
rs = json.load(open(f"{R}/results_rust_1core.json"))
rows = []
for m, cases in hf.items():
    if m not in rs:
        continue
    keys = [k for k in cases if k in rs[m]["cases"]]
    if not keys:
        continue
    py = [cases[k]["full_ns"] for k in keys]
    jin = [cases[k]["jinja_ns"] for k in keys]
    tok = [cases[k]["tok_ns"] for k in keys]
    cold = [rs[m]["cases"][k]["cold_ns"] for k in keys]
    incr = [rs[m]["cases"][k]["incr_ns"] for k in keys]
    toks = [cases[k]["tokens"] for k in keys]
    rows.append(dict(model=m, n=len(keys), tokens=statistics.mean(toks), py=statistics.mean(py) / 1e3, jinja=statistics.mean(jin) / 1e3,
                     tok=statistics.mean(tok) / 1e3, cold=statistics.mean(cold) / 1e3, incr=statistics.mean(incr) / 1e3,
                     speed_cold=statistics.median([p / c for p, c in zip(py, cold)]), speed_incr=statistics.median([p / c for p, c in zip(py, incr)])))

lines = ["| model | cases | mean tokens | HF total (us) | of which Jinja | of which tokenize | view cold (us) | view incr (us) | speedup cold (median) | speedup incr (median) |",
         "|---|---|---|---|---|---|---|---|---|---|"]
for r in rows:
    lines.append(f"| {r['model']} | {r['n']} | {r['tokens']:.0f} | {r['py']:.0f} | {r['jinja']:.0f} | {r['tok']:.0f} | {r['cold']:.0f} | {r['incr']:.0f} | {r['speed_cold']:.2f}x | {r['speed_incr']:.1f}x |")
allpy = sum(r["py"] * r["n"] for r in rows) / sum(r["n"] for r in rows)
allcold = sum(r["cold"] * r["n"] for r in rows) / sum(r["n"] for r in rows)
allincr = sum(r["incr"] * r["n"] for r in rows) / sum(r["n"] for r in rows)
summary = (f"\nAll {len(rows)} Jinja models, {sum(r['n'] for r in rows)} renders: HF mean {allpy:.0f} us, view cold {allcold:.0f} us "
           f"({allpy / allcold:.2f}x), view incr {allincr:.0f} us ({allpy / allincr:.1f}x). Median of per-model median speedups: cold "
           f"{statistics.median(r['speed_cold'] for r in rows):.2f}x, incr {statistics.median(r['speed_incr'] for r in rows):.1f}x.\n")
open(f"{R}/compare.md", "w").write("\n".join(lines) + "\n" + summary)
print(summary)

rows.sort(key=lambda r: r["py"])
fig, ax = plt.subplots(figsize=(9, 11))
y = range(len(rows))
ax.barh([i + 0.27 for i in y], [r["py"] for r in rows], height=0.27, label="HF apply_chat_template (Jinja2 + tokenize)", color="#9e9e9e")
ax.barh(list(y), [r["cold"] for r in rows], height=0.27, label="view renderer, cold (no cache)", color="#1f77b4")
ax.barh([i - 0.27 for i in y], [r["incr"] for r in rows], height=0.27, label="view renderer, next turn (run cache)", color="#2ca02c")
ax.set_yticks(list(y))
ax.set_yticklabels([r["model"].split("__")[1] for r in rows], fontsize=7)
ax.set_xlabel("mean microseconds per render, one core (lower is better)")
ax.set_title("Prompt rendering time, same conversations (dev + held-out 3)")
ax.legend(loc="lower right", fontsize=8)
plt.tight_layout()
plt.savefig(f"{R}/speed.png", dpi=130)
