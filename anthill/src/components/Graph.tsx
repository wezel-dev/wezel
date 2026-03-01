import { memo, useEffect, useMemo } from "react";
import {
  ReactFlow,
  Background,
  Controls,
  Handle,
  Position,
  BackgroundVariant,
  MarkerType,
  useReactFlow,
  ReactFlowProvider,
  type Node,
  type Edge,
  type NodeProps,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";

import { MONO } from "../lib/format";
import type { CrateTopo } from "../lib/data";
import type { HeatFn } from "../lib/theme";

// ── Graph layout ─────────────────────────────────────────────────────────────

interface LayoutNode {
  name: string;
  deps: string[];
  heat: number;
  external?: boolean;
}

export function layoutGraph(
  topo: CrateTopo[],
  heat: Record<string, number>,
  heatColor: HeatFn,
  highlightedCrates?: Set<string>,
  accentColor?: string,
): { nodes: Node[]; edges: Edge[] } {
  const items: LayoutNode[] = topo.map((c) => ({
    ...c,
    heat: heat[c.name] ?? 0,
  }));

  // O(1) lookup by name
  const nameToItem = new Map<string, LayoutNode>();
  const nameSet = new Set<string>();
  for (const c of items) {
    nameToItem.set(c.name, c);
    nameSet.add(c.name);
  }

  // Compute depths with iterative memoization
  const depths = new Map<string, number>();
  function getDepth(name: string): number {
    if (depths.has(name)) return depths.get(name)!;
    // Mark with -1 to detect cycles
    depths.set(name, 0);
    const node = nameToItem.get(name);
    if (!node || node.deps.length === 0) return 0;
    let maxChildDepth = -1;
    for (const d of node.deps) {
      if (nameSet.has(d)) {
        const cd = getDepth(d);
        if (cd > maxChildDepth) maxChildDepth = cd;
      }
    }
    const depth = maxChildDepth >= 0 ? 1 + maxChildDepth : 0;
    depths.set(name, depth);
    return depth;
  }
  for (const c of items) getDepth(c.name);

  const maxDepth = items.length > 0 ? Math.max(...depths.values()) : 0;
  const layers: string[][] = Array.from({ length: maxDepth + 1 }, () => []);
  for (const c of items) {
    layers[maxDepth - (depths.get(c.name) ?? 0)].push(c.name);
  }

  const NW = 150,
    NH = 44,
    GX = 32,
    GY = 72;

  // Pre-compute and deduplicate edge marker definitions keyed by color
  const markerCache = new Map<
    string,
    { type: MarkerType; color: string; width: number; height: number }
  >();
  function getMarker(color: string) {
    let m = markerCache.get(color);
    if (!m) {
      m = { type: MarkerType.ArrowClosed, color, width: 12, height: 12 };
      markerCache.set(color, m);
    }
    return m;
  }

  // Pre-compute heatColor results per unique heat value
  const colorCache = new Map<number, ReturnType<HeatFn>>();
  function getCachedColor(h: number) {
    let c = colorCache.get(h);
    if (!c) {
      c = heatColor(h);
      colorCache.set(h, c);
    }
    return c;
  }

  const nodes: Node[] = [];
  const edges: Edge[] = [];
  nodes.length = items.length; // pre-allocate
  let ni = 0;

  for (let ly = 0; ly < layers.length; ly++) {
    const layer = layers[ly];
    const w = layer.length * NW + (layer.length - 1) * GX;
    for (let ci = 0; ci < layer.length; ci++) {
      const name = layer[ci];
      const item = nameToItem.get(name)!;
      const isExternal = item.external ?? false;
      const colors = getCachedColor(item.heat);
      nodes[ni++] = {
        id: name,
        type: "crate",
        position: { x: -w / 2 + ci * (NW + GX), y: ly * (NH + GY) },
        data: {
          label: name,
          heat: item.heat,
          colors,
          highlighted: highlightedCrates?.has(name) ?? false,
          accentColor,
          external: isExternal,
        },
      };
    }
  }
  nodes.length = ni;

  // Build edges
  for (const crate of items) {
    const col = getCachedColor(crate.heat);
    const style = { stroke: col.border, strokeWidth: 1.5, opacity: 0.3 };
    const marker = getMarker(col.border);
    for (const dep of crate.deps) {
      if (nameSet.has(dep)) {
        edges.push({
          id: `${crate.name}->${dep}`,
          source: crate.name,
          target: dep,
          style,
          markerEnd: marker,
        });
      }
    }
  }

  return { nodes, edges };
}

// ── ReactFlow crate node ─────────────────────────────────────────────────────

interface CrateNodeData {
  label: string;
  heat: number;
  colors: { border: string; bg: string; text: string };
  highlighted?: boolean;
  accentColor?: string;
  external?: boolean;
}

const CrateNodeComponent = memo(function CrateNodeComponent({
  data,
}: NodeProps) {
  const d = data as unknown as CrateNodeData;
  const hl = d.highlighted && d.accentColor;
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
        outline: hl ? `2.5px solid ${d.accentColor}` : "none",
        outlineOffset: 2,
        transition: "outline 0.15s",
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
      {!d.external && (
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
      )}
      <div>
        {d.external ? "📦 " : ""}
        {d.label}
      </div>
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
});

const nodeTypes = { crate: CrateNodeComponent };

// ── Graph wrapper that re-fits on resize ─────────────────────────────────────

function FitViewOnResize() {
  const { fitView } = useReactFlow();

  useEffect(() => {
    const el = document.querySelector(".react-flow") as HTMLElement | null;
    if (!el) return;
    const ro = new ResizeObserver(() => {
      fitView({ padding: 0.25, duration: 150 });
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, [fitView]);

  return null;
}

export function FitViewGraph({
  nodes,
  edges,
  colorMode,
  bg,
  surface,
  border,
  onNodeClick,
}: {
  nodes: Node[];
  edges: Edge[];
  colorMode: "light" | "dark";
  bg: string;
  surface: string;
  border: string;
  onNodeClick?: (crateName: string) => void;
}) {
  const handleNodeClick = useMemo(
    () =>
      onNodeClick
        ? (_event: React.MouseEvent, node: Node) => onNodeClick(node.id)
        : undefined,
    [onNodeClick],
  );

  return (
    <ReactFlowProvider>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        nodeTypes={nodeTypes}
        onNodeClick={handleNodeClick}
        fitView
        fitViewOptions={{ padding: 0.25 }}
        colorMode={colorMode}
        minZoom={0.3}
        maxZoom={2}
        proOptions={{ hideAttribution: true }}
      >
        <FitViewOnResize />
        <Background
          variant={BackgroundVariant.Dots}
          gap={16}
          size={1}
          color={bg}
        />
        <Controls
          style={{
            background: surface,
            borderRadius: 4,
            border: `1px solid ${border}`,
          }}
        />
      </ReactFlow>
    </ReactFlowProvider>
  );
}

export { nodeTypes };
