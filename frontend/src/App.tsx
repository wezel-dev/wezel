import { useState, useCallback, useMemo } from "react";
import { Workflow, Search, X, Pin, PinOff } from "lucide-react";
import {
  ReactFlow,
  Background,
  Controls,
  MiniMap,
  Handle,
  Position,
  BackgroundVariant,
  MarkerType,
  type Node,
  type Edge,
  type NodeProps,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";

import usersData from "./mock_data/users.json";
import scenariosData from "./mock_data/scenarios.json";
import graph1 from "./mock_data/graphs/1.json";
import graph2 from "./mock_data/graphs/2.json";
import graph3 from "./mock_data/graphs/3.json";
import graph4 from "./mock_data/graphs/4.json";
import graph5 from "./mock_data/graphs/5.json";
import graph6 from "./mock_data/graphs/6.json";
import graph7 from "./mock_data/graphs/7.json";
import graph8 from "./mock_data/graphs/8.json";
import runs1 from "./mock_data/runs/1.json";
import runs2 from "./mock_data/runs/2.json";
import runs3 from "./mock_data/runs/3.json";
import runs4 from "./mock_data/runs/4.json";
import runs5 from "./mock_data/runs/5.json";
import runs6 from "./mock_data/runs/6.json";
import runs7 from "./mock_data/runs/7.json";
import runs8 from "./mock_data/runs/8.json";

// ── Data model ───────────────────────────────────────────────────────────────

interface CrateTopo {
  name: string;
  deps: string[];
}

interface Run {
  user: string;
  timestamp: string;
  buildTimeMs: number;
  dirtyCrates: string[];
}

interface Scenario {
  id: number;
  name: string;
  profile: "dev" | "release";
  pinned: boolean;
  graph: CrateTopo[];
  runs: Run[];
}

// ── Heat computation ─────────────────────────────────────────────────────────

/** Given a set of runs and the full crate list, compute heat per crate (0–100) */
function computeHeat(
  runs: Run[],
  crateNames: string[],
): Record<string, number> {
  if (runs.length === 0) {
    return Object.fromEntries(crateNames.map((n) => [n, 0]));
  }
  const counts: Record<string, number> = {};
  for (const name of crateNames) counts[name] = 0;
  for (const run of runs) {
    for (const c of run.dirtyCrates) {
      if (c in counts) counts[c]++;
    }
  }
  const result: Record<string, number> = {};
  for (const name of crateNames) {
    result[name] = Math.round((counts[name] / runs.length) * 100);
  }
  return result;
}

function heatColor(heat: number): { border: string; bg: string; text: string } {
  if (heat >= 80) return { border: "#ef4444", bg: "#3b1118", text: "#fca5a5" };
  if (heat >= 60) return { border: "#f59e0b", bg: "#352008", text: "#fcd34d" };
  if (heat >= 40) return { border: "#eab308", bg: "#2e2a08", text: "#fde68a" };
  if (heat >= 20) return { border: "#6366f1", bg: "#1c1a3a", text: "#a5b4fc" };
  return { border: "#334155", bg: "#111827", text: "#64748b" };
}

// ── Mock data (from JSON) ────────────────────────────────────────────────────

const USERS: string[] = usersData;

const graphsById: Record<number, CrateTopo[]> = {
  1: graph1,
  2: graph2,
  3: graph3,
  4: graph4,
  5: graph5,
  6: graph6,
  7: graph7,
  8: graph8,
};
const runsById: Record<number, Run[]> = {
  1: runs1 as Run[],
  2: runs2 as Run[],
  3: runs3 as Run[],
  4: runs4 as Run[],
  5: runs5 as Run[],
  6: runs6 as Run[],
  7: runs7 as Run[],
  8: runs8 as Run[],
};

const MOCK_SCENARIOS: Scenario[] = (
  scenariosData as {
    id: number;
    name: string;
    profile: "dev" | "release";
    pinned: boolean;
  }[]
).map((s) => ({
  ...s,
  graph: graphsById[s.id] ?? [],
  runs: runsById[s.id] ?? [],
}));

// ── Theme ────────────────────────────────────────────────────────────────────

const C = {
  bg: "#0a0a14",
  surface: "#12121f",
  surface2: "#1a1a2e",
  surface3: "#22223a",
  border: "#262640",
  text: "#d4d4e8",
  textMid: "#9494b8",
  textDim: "#5a5a7a",
  accent: "#6366f1",
  green: "#22c55e",
  amber: "#f59e0b",
  red: "#ef4444",
  pink: "#ec4899",
  cyan: "#06b6d4",
};

const MONO = "'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace";
const SANS = "'Inter', -apple-system, system-ui, sans-serif";

// ── Helpers ──────────────────────────────────────────────────────────────────

function fmtMs(ms: number): string {
  if (ms >= 60_000) return `${(ms / 60_000).toFixed(1)}m`;
  if (ms >= 1000) return `${(ms / 1000).toFixed(1)}s`;
  return `${ms}ms`;
}

function fmtTime(ts: string): string {
  const d = new Date(ts);
  const mon = (d.getMonth() + 1).toString().padStart(2, "0");
  const day = d.getDate().toString().padStart(2, "0");
  const h = d.getHours().toString().padStart(2, "0");
  const m = d.getMinutes().toString().padStart(2, "0");
  return `${mon}/${day} ${h}:${m}`;
}

// ── Graph layout ─────────────────────────────────────────────────────────────

interface LayoutNode {
  name: string;
  deps: string[];
  heat: number;
}

function layoutGraph(
  topo: CrateTopo[],
  heat: Record<string, number>,
): { nodes: Node[]; edges: Edge[] } {
  const items: LayoutNode[] = topo.map((c) => ({
    ...c,
    heat: heat[c.name] ?? 0,
  }));

  const nameToIdx = new Map<string, number>();
  items.forEach((c, i) => nameToIdx.set(c.name, i));

  const depths = new Map<string, number>();
  function getDepth(name: string): number {
    if (depths.has(name)) return depths.get(name)!;
    const node = items.find((c) => c.name === name);
    if (!node || node.deps.length === 0) {
      depths.set(name, 0);
      return 0;
    }
    const d =
      1 +
      Math.max(
        ...node.deps.filter((d) => nameToIdx.has(d)).map((d) => getDepth(d)),
      );
    depths.set(name, d);
    return d;
  }
  items.forEach((c) => getDepth(c.name));

  const maxDepth = Math.max(...Array.from(depths.values()), 0);
  const layers: string[][] = Array.from({ length: maxDepth + 1 }, () => []);
  items.forEach((c) => {
    layers[maxDepth - (depths.get(c.name) ?? 0)].push(c.name);
  });

  const NW = 150,
    NH = 44,
    GX = 32,
    GY = 72;
  const nodes: Node[] = [];
  const edges: Edge[] = [];

  layers.forEach((layer, ly) => {
    const w = layer.length * NW + (layer.length - 1) * GX;
    layer.forEach((name, ci) => {
      const item = items.find((c) => c.name === name)!;
      const colors = heatColor(item.heat);
      nodes.push({
        id: name,
        type: "crate",
        position: { x: -w / 2 + ci * (NW + GX), y: ly * (NH + GY) },
        data: { label: name, heat: item.heat, colors },
      });
    });
  });

  items.forEach((crate) => {
    crate.deps.forEach((dep) => {
      if (nameToIdx.has(dep)) {
        const col = heatColor(crate.heat);
        edges.push({
          id: `${crate.name}->${dep}`,
          source: crate.name,
          target: dep,
          style: { stroke: col.border, strokeWidth: 1.5, opacity: 0.45 },
          markerEnd: {
            type: MarkerType.ArrowClosed,
            color: col.border,
            width: 12,
            height: 12,
          },
        });
      }
    });
  });

  return { nodes, edges };
}

// ── ReactFlow crate node ─────────────────────────────────────────────────────

function CrateNodeComponent({ data }: NodeProps) {
  const d = data as {
    label: string;
    heat: number;
    colors: { border: string; bg: string; text: string };
  };
  return (
    <div
      style={{
        background: d.colors.bg,
        border: `1.5px solid ${d.colors.border}`,
        borderRadius: 6,
        padding: "4px 10px",
        color: d.colors.text,
        fontSize: 11,
        fontFamily: MONO,
        fontWeight: 500,
        minWidth: 100,
        textAlign: "center",
        boxShadow: `0 0 8px ${d.colors.border}22`,
      }}
    >
      <Handle
        type="target"
        position={Position.Top}
        style={{
          background: d.colors.border,
          width: 5,
          height: 5,
          border: "none",
        }}
      />
      <div
        style={{
          fontSize: 8,
          color: d.colors.border,
          letterSpacing: 0.8,
          marginBottom: 1,
        }}
      >
        {d.heat}%
      </div>
      <div>{d.label}</div>
      <Handle
        type="source"
        position={Position.Bottom}
        style={{
          background: d.colors.border,
          width: 5,
          height: 5,
          border: "none",
        }}
      />
    </div>
  );
}

const nodeTypes = { crate: CrateNodeComponent };

// ── Small components ─────────────────────────────────────────────────────────

function FreqBar({ value, max }: { value: number; max: number }) {
  const pct = max > 0 ? Math.round((value / max) * 100) : 0;
  const col = pct >= 70 ? C.red : pct >= 40 ? C.amber : C.accent;
  return (
    <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
      <div
        style={{
          flex: 1,
          height: 4,
          background: C.surface3,
          borderRadius: 2,
          overflow: "hidden",
        }}
      >
        <div
          style={{
            width: `${pct}%`,
            height: "100%",
            background: col,
            borderRadius: 2,
          }}
        />
      </div>
      <span
        style={{
          fontSize: 10,
          color: col,
          minWidth: 24,
          textAlign: "right",
          fontFamily: MONO,
        }}
      >
        {value}
      </span>
    </div>
  );
}

function Badge({
  children,
  color,
  bg,
}: {
  children: React.ReactNode;
  color: string;
  bg: string;
}) {
  return (
    <span
      style={{
        fontSize: 10,
        fontWeight: 600,
        letterSpacing: 0.6,
        padding: "1px 6px",
        borderRadius: 3,
        background: bg,
        color,
        border: `1px solid ${color}33`,
        textTransform: "uppercase",
      }}
    >
      {children}
    </span>
  );
}

function Stat({
  label,
  value,
  color,
}: {
  label: string;
  value: string;
  color: string;
}) {
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 1 }}>
      <span
        style={{
          fontSize: 9,
          color: C.textDim,
          textTransform: "uppercase",
          letterSpacing: 0.8,
          fontWeight: 600,
        }}
      >
        {label}
      </span>
      <span style={{ fontSize: 15, fontWeight: 700, color, fontFamily: MONO }}>
        {value}
      </span>
    </div>
  );
}

// ── Filter bar ───────────────────────────────────────────────────────────────

function FilterBar({
  search,
  onSearch,
  userFilter,
  onUserFilter,
  profileFilter,
  onProfileFilter,
}: {
  search: string;
  onSearch: (v: string) => void;
  userFilter: string[];
  onUserFilter: (v: string[]) => void;
  profileFilter: string | null;
  onProfileFilter: (v: string | null) => void;
}) {
  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 8,
        padding: "6px 0",
        fontSize: 11,
        flexWrap: "wrap",
      }}
    >
      {/* Search */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 4,
          background: C.surface2,
          border: `1px solid ${C.border}`,
          borderRadius: 4,
          padding: "3px 8px",
          minWidth: 180,
        }}
      >
        <Search size={12} color={C.textDim} />
        <input
          value={search}
          onChange={(e) => onSearch(e.target.value)}
          placeholder="filter commands…"
          style={{
            background: "transparent",
            border: "none",
            outline: "none",
            color: C.text,
            fontSize: 11,
            fontFamily: MONO,
            width: "100%",
          }}
        />
        {search && (
          <button
            onClick={() => onSearch("")}
            style={{
              background: "none",
              border: "none",
              cursor: "pointer",
              padding: 0,
              display: "flex",
            }}
          >
            <X size={11} color={C.textDim} />
          </button>
        )}
      </div>

      {/* User filter */}
      <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
        <span
          style={{
            color: C.textDim,
            fontSize: 10,
            fontWeight: 600,
            letterSpacing: 0.5,
          }}
        >
          USER
        </span>
        {USERS.map((u) => (
          <button
            key={u}
            onClick={() =>
              onUserFilter(
                userFilter.includes(u)
                  ? userFilter.filter((x) => x !== u)
                  : [...userFilter, u],
              )
            }
            style={{
              background: userFilter.includes(u)
                ? C.accent + "22"
                : "transparent",
              border: `1px solid ${userFilter.includes(u) ? C.accent : C.border}`,
              borderRadius: 3,
              padding: "2px 7px",
              cursor: "pointer",
              color: userFilter.includes(u) ? C.accent : C.textMid,
              fontSize: 10,
              fontFamily: MONO,
            }}
          >
            {u}
          </button>
        ))}
      </div>

      {/* Profile filter */}
      <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
        <span
          style={{
            color: C.textDim,
            fontSize: 10,
            fontWeight: 600,
            letterSpacing: 0.5,
          }}
        >
          PROFILE
        </span>
        {(["dev", "release"] as const).map((p) => (
          <button
            key={p}
            onClick={() => onProfileFilter(profileFilter === p ? null : p)}
            style={{
              background: profileFilter === p ? C.accent + "22" : "transparent",
              border: `1px solid ${profileFilter === p ? C.accent : C.border}`,
              borderRadius: 3,
              padding: "2px 7px",
              cursor: "pointer",
              color: profileFilter === p ? C.accent : C.textMid,
              fontSize: 10,
              fontFamily: MONO,
              textTransform: "uppercase",
            }}
          >
            {p}
          </button>
        ))}
      </div>
    </div>
  );
}

// ── Heat legend ──────────────────────────────────────────────────────────────

function HeatLegend() {
  const stops = [
    { label: "cold", heat: 5 },
    { label: "low", heat: 25 },
    { label: "mid", heat: 45 },
    { label: "warm", heat: 65 },
    { label: "hot", heat: 90 },
  ];
  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 10,
        fontSize: 9,
        color: C.textDim,
        fontFamily: MONO,
      }}
    >
      <span
        style={{
          fontWeight: 700,
          letterSpacing: 0.5,
          textTransform: "uppercase",
        }}
      >
        rebuild freq
      </span>
      {stops.map((s) => {
        const c = heatColor(s.heat);
        return (
          <div
            key={s.label}
            style={{ display: "flex", alignItems: "center", gap: 3 }}
          >
            <div
              style={{
                width: 8,
                height: 8,
                borderRadius: 2,
                background: c.bg,
                border: `1.5px solid ${c.border}`,
              }}
            />
            <span style={{ color: c.text }}>{s.label}</span>
          </div>
        );
      })}
    </div>
  );
}

// ── Run list (left panel inside detail view) ─────────────────────────────────

function RunList({
  runs,
  selectedIndices,
  onToggle,
  onSelectAll,
  onSelectNone,
}: {
  runs: Run[];
  selectedIndices: Set<number>;
  onToggle: (i: number) => void;
  onSelectAll: () => void;
  onSelectNone: () => void;
}) {
  const allSelected = selectedIndices.size === runs.length;

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        height: "100%",
        minWidth: 220,
        width: 240,
        borderRight: `1px solid ${C.border}`,
        flexShrink: 0,
      }}
    >
      {/* Header */}
      <div
        style={{
          padding: "6px 10px",
          borderBottom: `1px solid ${C.border}`,
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
        }}
      >
        <span
          style={{
            fontSize: 9,
            fontWeight: 700,
            color: C.textDim,
            letterSpacing: 0.8,
            textTransform: "uppercase",
          }}
        >
          Runs ({selectedIndices.size}/{runs.length})
        </span>
        <div style={{ display: "flex", gap: 6 }}>
          <button
            onClick={allSelected ? onSelectNone : onSelectAll}
            style={{
              background: "none",
              border: `1px solid ${C.border}`,
              borderRadius: 3,
              padding: "1px 6px",
              cursor: "pointer",
              color: C.textMid,
              fontSize: 9,
              fontFamily: MONO,
            }}
          >
            {allSelected ? "none" : "all"}
          </button>
        </div>
      </div>

      {/* Run rows */}
      <div style={{ flex: 1, overflowY: "auto" }}>
        {runs.map((run, i) => {
          const isSel = selectedIndices.has(i);
          return (
            <div
              key={i}
              onClick={() => onToggle(i)}
              style={{
                display: "flex",
                alignItems: "center",
                gap: 6,
                padding: "4px 10px",
                cursor: "pointer",
                background: isSel ? C.accent + "10" : "transparent",
                borderLeft: isSel
                  ? `2px solid ${C.accent}`
                  : "2px solid transparent",
                transition: "all 0.08s",
                fontSize: 10,
              }}
              onMouseEnter={(e) => {
                if (!isSel) e.currentTarget.style.background = C.surface2;
              }}
              onMouseLeave={(e) => {
                if (!isSel) e.currentTarget.style.background = "transparent";
              }}
            >
              {/* Checkbox */}
              <div
                style={{
                  width: 12,
                  height: 12,
                  borderRadius: 2,
                  border: `1.5px solid ${isSel ? C.accent : C.border}`,
                  background: isSel ? C.accent + "33" : "transparent",
                  flexShrink: 0,
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                  fontSize: 8,
                  color: C.accent,
                }}
              >
                {isSel ? "✓" : ""}
              </div>
              {/* User */}
              <span style={{ color: C.cyan, fontFamily: MONO, minWidth: 40 }}>
                {run.user}
              </span>
              {/* Timestamp */}
              <span style={{ color: C.textDim, fontFamily: MONO, flex: 1 }}>
                {fmtTime(run.timestamp)}
              </span>
              {/* Build time */}
              <span style={{ color: C.textMid, fontFamily: MONO }}>
                {fmtMs(run.buildTimeMs)}
              </span>
              {/* Dirty count */}
              <span
                style={{
                  color: C.amber,
                  fontFamily: MONO,
                  fontSize: 9,
                  minWidth: 16,
                  textAlign: "right",
                }}
              >
                {run.dirtyCrates.length}
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
}

// ── Summary panel ────────────────────────────────────────────────────────────

function Summary({
  scenario,
  selectedRuns,
  heat,
}: {
  scenario: Scenario;
  selectedRuns: Run[];
  heat: Record<string, number>;
}) {
  const crateNames = scenario.graph.map((c) => c.name);
  const hotCrates = crateNames
    .map((n) => ({ name: n, heat: heat[n] ?? 0 }))
    .sort((a, b) => b.heat - a.heat)
    .slice(0, 8);

  const avgBuild =
    selectedRuns.length > 0
      ? Math.round(
          selectedRuns.reduce((s, r) => s + r.buildTimeMs, 0) /
            selectedRuns.length,
        )
      : 0;

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
          value={selectedRuns.length > 0 ? fmtMs(avgBuild) : "—"}
          color={C.amber}
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

// ── Detail view ──────────────────────────────────────────────────────────────

function DetailView({ scenario }: { scenario: Scenario }) {
  // All runs selected by default
  const [selectedIndices, setSelectedIndices] = useState<Set<number>>(
    () => new Set(scenario.runs.map((_, i) => i)),
  );

  const toggleRun = useCallback((i: number) => {
    setSelectedIndices((prev) => {
      const next = new Set(prev);
      if (next.has(i)) next.delete(i);
      else next.add(i);
      return next;
    });
  }, []);

  const selectAll = useCallback(() => {
    setSelectedIndices(new Set(scenario.runs.map((_, i) => i)));
  }, [scenario.runs]);

  const selectNone = useCallback(() => {
    setSelectedIndices(new Set());
  }, []);

  const selectedRuns = useMemo(
    () => scenario.runs.filter((_, i) => selectedIndices.has(i)),
    [scenario.runs, selectedIndices],
  );

  const crateNames = useMemo(
    () => scenario.graph.map((c) => c.name),
    [scenario.graph],
  );

  const heat = useMemo(
    () => computeHeat(selectedRuns, crateNames),
    [selectedRuns, crateNames],
  );

  const { nodes, edges } = useMemo(
    () => layoutGraph(scenario.graph, heat),
    [scenario.graph, heat],
  );

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        height: "100%",
        gap: 8,
      }}
    >
      {/* Header row */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          gap: 12,
          flexWrap: "wrap",
          flexShrink: 0,
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <span
            style={{
              fontSize: 13,
              fontWeight: 600,
              color: C.text,
              fontFamily: MONO,
            }}
          >
            {scenario.name}
          </span>
          <Badge
            color={scenario.profile === "dev" ? C.textMid : C.amber}
            bg={scenario.profile === "dev" ? C.surface3 : C.amber + "18"}
          >
            {scenario.profile}
          </Badge>
          {scenario.pinned && (
            <span style={{ fontSize: 10, color: C.accent }}>📌 tracked</span>
          )}
        </div>
        <HeatLegend />
      </div>

      {/* Body: runs list | graph | summary */}
      <div style={{ flex: 1, display: "flex", gap: 0, minHeight: 0 }}>
        {/* Run list */}
        <RunList
          runs={scenario.runs}
          selectedIndices={selectedIndices}
          onToggle={toggleRun}
          onSelectAll={selectAll}
          onSelectNone={selectNone}
        />

        {/* Graph */}
        <div
          style={{
            flex: 1,
            borderRadius: 0,
            overflow: "hidden",
            background: C.bg,
          }}
        >
          <ReactFlow
            nodes={nodes}
            edges={edges}
            nodeTypes={nodeTypes}
            fitView
            fitViewOptions={{ padding: 0.25 }}
            colorMode="dark"
            minZoom={0.3}
            maxZoom={2}
            proOptions={{ hideAttribution: true }}
          >
            <Background
              variant={BackgroundVariant.Dots}
              gap={16}
              size={1}
              color="#1a1a2e"
            />
            <Controls
              style={{
                background: C.surface,
                borderRadius: 4,
                border: `1px solid ${C.border}`,
              }}
            />
            <MiniMap
              nodeColor={(n) => {
                const c = (n.data as { colors?: { border: string } })?.colors;
                return c?.border ?? C.accent;
              }}
              maskColor="rgba(0,0,0,0.75)"
              style={{
                background: C.bg,
                border: `1px solid ${C.border}`,
                borderRadius: 4,
                height: 80,
                width: 120,
              }}
            />
          </ReactFlow>
        </div>

        {/* Summary sidebar */}
        <div
          style={{
            width: 190,
            overflowY: "auto",
            padding: "8px 10px",
            borderLeft: `1px solid ${C.border}`,
            flexShrink: 0,
          }}
        >
          <Summary
            scenario={scenario}
            selectedRuns={selectedRuns}
            heat={heat}
          />
        </div>
      </div>
    </div>
  );
}

// ── App ──────────────────────────────────────────────────────────────────────

export default function App() {
  const [scenarios, setScenarios] = useState(MOCK_SCENARIOS);
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [search, setSearch] = useState("");
  const [userFilter, setUserFilter] = useState<string[]>([]);
  const [profileFilter, setProfileFilter] = useState<string | null>(null);

  const togglePin = useCallback((id: number) => {
    setScenarios((prev) =>
      prev.map((s) => (s.id === id ? { ...s, pinned: !s.pinned } : s)),
    );
  }, []);

  /** Count runs matching user filter for a scenario */
  const getFreq = useCallback(
    (s: Scenario) => {
      if (userFilter.length === 0) return s.runs.length;
      return s.runs.filter((r) => userFilter.includes(r.user)).length;
    },
    [userFilter],
  );

  const filtered = useMemo(() => {
    let list = [...scenarios];
    if (search) {
      const q = search.toLowerCase();
      list = list.filter((s) => s.name.toLowerCase().includes(q));
    }
    if (profileFilter) list = list.filter((s) => s.profile === profileFilter);
    list.sort((a, b) => getFreq(b) - getFreq(a));
    return list;
  }, [scenarios, search, profileFilter, getFreq]);

  const maxFreq = useMemo(
    () => Math.max(...filtered.map(getFreq), 1),
    [filtered, getFreq],
  );

  const selected =
    selectedId != null
      ? (scenarios.find((s) => s.id === selectedId) ?? null)
      : null;

  return (
    <div
      style={{
        width: "100vw",
        height: "100vh",
        background: C.bg,
        color: C.text,
        fontFamily: SANS,
        display: "flex",
        flexDirection: "column",
        overflow: "hidden",
      }}
    >
      {/* ── Top bar ──────────────────────────────────────── */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          padding: "0 16px",
          height: 40,
          minHeight: 40,
          borderBottom: `1px solid ${C.border}`,
          background: C.surface,
          justifyContent: "space-between",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <Workflow size={18} color={C.accent} strokeWidth={2.5} />
          <span
            style={{
              fontSize: 15,
              fontWeight: 800,
              color: C.accent,
              letterSpacing: -0.5,
            }}
          >
            wezel
          </span>
        </div>
        <div style={{ fontSize: 10, color: C.textDim, fontFamily: MONO }}>
          {filtered.length}/{scenarios.length} commands ·{" "}
          {scenarios.filter((s) => s.pinned).length} tracked
        </div>
      </div>

      {/* ── Main ─────────────────────────────────────────── */}
      <div style={{ flex: 1, display: "flex", overflow: "hidden" }}>
        {/* Left: command list */}
        <div
          style={{
            width: selected ? 380 : "100%",
            minWidth: 340,
            flexShrink: 0,
            display: "flex",
            flexDirection: "column",
            borderRight: selected ? `1px solid ${C.border}` : "none",
            transition: "width 0.15s ease",
          }}
        >
          {/* Filters */}
          <div
            style={{
              padding: "6px 12px",
              borderBottom: `1px solid ${C.border}`,
            }}
          >
            <FilterBar
              search={search}
              onSearch={setSearch}
              userFilter={userFilter}
              onUserFilter={setUserFilter}
              profileFilter={profileFilter}
              onProfileFilter={setProfileFilter}
            />
          </div>

          {/* Table header */}
          <div
            style={{
              display: "grid",
              gridTemplateColumns:
                "minmax(140px, 3fr) 50px minmax(80px, 1fr) 56px",
              gap: 6,
              padding: "4px 12px",
              fontSize: 9,
              fontWeight: 700,
              color: C.textDim,
              textTransform: "uppercase",
              letterSpacing: 0.8,
              borderBottom: `1px solid ${C.border}`,
              background: C.surface,
            }}
          >
            <span>Command</span>
            <span>Prof.</span>
            <span>Runs</span>
            <span style={{ textAlign: "center" }}>Track</span>
          </div>

          {/* Rows */}
          <div style={{ flex: 1, overflowY: "auto" }}>
            {filtered.length === 0 && (
              <div
                style={{
                  padding: 20,
                  textAlign: "center",
                  color: C.textDim,
                  fontSize: 12,
                }}
              >
                No commands match filters
              </div>
            )}
            {filtered.map((s) => {
              const isSel = s.id === selectedId;
              const freq = getFreq(s);
              return (
                <div
                  key={s.id}
                  onClick={() => setSelectedId(isSel ? null : s.id)}
                  style={{
                    display: "grid",
                    gridTemplateColumns:
                      "minmax(140px, 3fr) 50px minmax(80px, 1fr) 56px",
                    gap: 6,
                    padding: "6px 12px",
                    alignItems: "center",
                    cursor: "pointer",
                    background: isSel ? C.accent + "10" : "transparent",
                    borderLeft: isSel
                      ? `2px solid ${C.accent}`
                      : "2px solid transparent",
                    transition: "all 0.1s",
                  }}
                  onMouseEnter={(e) => {
                    if (!isSel) e.currentTarget.style.background = C.surface2;
                  }}
                  onMouseLeave={(e) => {
                    if (!isSel)
                      e.currentTarget.style.background = "transparent";
                  }}
                >
                  <span
                    style={{
                      fontSize: 11,
                      fontWeight: 500,
                      color: isSel ? C.text : C.textMid,
                      fontFamily: MONO,
                      overflow: "hidden",
                      textOverflow: "ellipsis",
                      whiteSpace: "nowrap",
                    }}
                  >
                    {s.name}
                  </span>
                  <Badge
                    color={s.profile === "dev" ? C.textDim : C.amber}
                    bg={s.profile === "dev" ? C.surface3 : C.amber + "15"}
                  >
                    {s.profile === "dev" ? "dev" : "rel"}
                  </Badge>
                  <FreqBar value={freq} max={maxFreq} />
                  <div style={{ display: "flex", justifyContent: "center" }}>
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        togglePin(s.id);
                      }}
                      style={{
                        background: "none",
                        border: "none",
                        cursor: "pointer",
                        padding: 2,
                        color: s.pinned ? C.accent : C.textDim,
                        display: "flex",
                        opacity: s.pinned ? 1 : 0.5,
                      }}
                    >
                      {s.pinned ? <Pin size={13} /> : <PinOff size={13} />}
                    </button>
                  </div>
                </div>
              );
            })}
          </div>
        </div>

        {/* Right: detail */}
        {selected && (
          <div
            style={{
              flex: 1,
              padding: 12,
              overflow: "hidden",
              background: C.bg,
            }}
          >
            <DetailView key={selected.id} scenario={selected} />
          </div>
        )}
      </div>
    </div>
  );
}
