import { useTheme } from "../lib/theme";
import { MONO } from "../lib/format";
import { fmtMs } from "../lib/format";
import { Stat } from "./Stat";
import type { Scenario, Run } from "../lib/data";

function percentile(sorted: number[], p: number): number {
  if (sorted.length === 0) return 0;
  const idx = (p / 100) * (sorted.length - 1);
  const lo = Math.floor(idx);
  const hi = Math.ceil(idx);
  if (lo === hi) return sorted[lo];
  return sorted[lo] + (sorted[hi] - sorted[lo]) * (idx - lo);
}

function fmtMsPrecise(ms: number): string {
  if (ms >= 3_600_000) {
    const h = Math.floor(ms / 3_600_000);
    const m = Math.floor((ms % 3_600_000) / 60_000);
    const s = ((ms % 60_000) / 1000).toFixed(1);
    return `${h}h ${m}m ${s}s`;
  }
  if (ms >= 60_000) {
    const m = Math.floor(ms / 60_000);
    const s = ((ms % 60_000) / 1000).toFixed(2);
    return `${m}m ${s}s`;
  }
  if (ms >= 1000) return `${(ms / 1000).toFixed(3)}s`;
  return `${ms.toFixed(1)}ms`;
}

function fmtTimespan(runs: Run[]): string {
  if (runs.length === 0) return "—";
  const times = runs.map((r) => {
    const ts = r.timestamp;
    return /^\d+$/.test(ts) ? Number(ts) * 1000 : new Date(ts).getTime();
  });
  const min = Math.min(...times);
  const max = Math.max(...times);
  const diffMs = max - min;
  if (diffMs < 60_000) return `${(diffMs / 1000).toFixed(0)}s`;
  if (diffMs < 3_600_000) return `${(diffMs / 60_000).toFixed(1)}m`;
  if (diffMs < 86_400_000) return `${(diffMs / 3_600_000).toFixed(1)}h`;
  return `${(diffMs / 86_400_000).toFixed(1)}d`;
}

export function Summary({
  scenario,
  selectedRuns,
  heat,
}: {
  scenario: Scenario;
  selectedRuns: Run[];
  heat: Record<string, number>;
}) {
  const { C, heatColor } = useTheme();
  const crateNames = scenario.graph.map((c) => c.name);
  const hotCrates = crateNames
    .map((n) => ({ name: n, heat: heat[n] ?? 0 }))
    .sort((a, b) => b.heat - a.heat)
    .slice(0, 8);

  const sorted = selectedRuns
    .map((r) => r.buildTimeMs)
    .slice()
    .sort((a, b) => a - b);

  const n = sorted.length;
  const avg = n > 0 ? Math.round(sorted.reduce((s, v) => s + v, 0) / n) : 0;
  const med = n > 0 ? Math.round(percentile(sorted, 50)) : 0;
  const p75 = n > 0 ? Math.round(percentile(sorted, 75)) : 0;
  const p90 = n > 0 ? Math.round(percentile(sorted, 90)) : 0;
  const sum = sorted.reduce((s, v) => s + v, 0);

  const dash = "—";

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        gap: 12,
        fontSize: 11,
        minWidth: 170,
      }}
    >
      {/* Metrics */}
      <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
        <Stat
          label="Avg build"
          value={n > 0 ? fmtMs(avg) : dash}
          color={C.amber}
        />
        <Stat
          label="Median"
          value={n > 0 ? fmtMs(med) : dash}
          color={C.amber}
        />
        <Stat label="p75" value={n > 0 ? fmtMs(p75) : dash} color={C.amber} />
        <Stat label="p90" value={n > 0 ? fmtMs(p90) : dash} color={C.red} />
      </div>

      <div style={{ height: 1, background: C.border }} />

      <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
        <Stat
          label="Total time"
          value={n > 0 ? fmtMsPrecise(sum) : dash}
          color={C.cyan}
        />
        <Stat
          label="Timespan"
          value={fmtTimespan(selectedRuns)}
          color={C.textMid}
        />
        <Stat
          label="Runs selected"
          value={`${selectedRuns.length}/${scenario.runs.length}`}
          color={C.accent}
        />
        <Stat
          label="Crates in graph"
          value={`${scenario.graph.length}`}
          color={C.pink}
        />
      </div>

      <div style={{ height: 1, background: C.border }} />

      {/* Hottest crates */}
      <div>
        <div
          style={{
            fontSize: 9,
            fontWeight: 700,
            color: C.textDim,
            letterSpacing: 0.8,
            textTransform: "uppercase",
            marginBottom: 6,
          }}
        >
          Rebuild frequency
        </div>
        <div style={{ display: "flex", flexDirection: "column", gap: 3 }}>
          {hotCrates.map((c) => {
            const col = heatColor(c.heat);
            return (
              <div
                key={c.name}
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: 6,
                  padding: "3px 6px",
                  borderRadius: 3,
                  background: col.bg,
                  border: `1px solid ${col.border}33`,
                }}
              >
                <span
                  style={{
                    fontSize: 10,
                    fontFamily: MONO,
                    color: col.text,
                    flex: 1,
                    overflow: "hidden",
                    textOverflow: "ellipsis",
                    whiteSpace: "nowrap",
                  }}
                >
                  {c.name}
                </span>
                <span
                  style={{
                    fontSize: 9,
                    fontFamily: MONO,
                    color: col.border,
                    fontWeight: 700,
                    minWidth: 28,
                    textAlign: "right",
                  }}
                >
                  {c.heat}%
                </span>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
