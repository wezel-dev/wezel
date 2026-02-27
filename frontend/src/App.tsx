import {
  ReactFlow,
  Background,
  Controls,
  MiniMap,
  useNodesState,
  useEdgesState,
  type Node,
  type Edge,
  type NodeProps,
  Handle,
  Position,
  BackgroundVariant,
  MarkerType,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
const CATEGORIES = [
  { name: "Ingest", color: "#6366f1", bg: "#1e1b4b" },
  { name: "Transform", color: "#f59e0b", bg: "#451a03" },
  { name: "Validate", color: "#10b981", bg: "#022c22" },
  { name: "Enrich", color: "#ec4899", bg: "#500724" },
  { name: "Load", color: "#06b6d4", bg: "#083344" },
  { name: "Publish", color: "#8b5cf6", bg: "#2e1065" },
  { name: "Archive", color: "#f43f5e", bg: "#4c0519" },
  { name: "Monitor", color: "#84cc16", bg: "#1a2e05" },
];

const TASK_NAMES = [
  "fetch",
  "parse",
  "filter",
  "map",
  "reduce",
  "merge",
  "split",
  "dedupe",
  "sort",
  "aggregate",
  "join",
  "pivot",
  "normalize",
  "encode",
  "decode",
  "compress",
  "validate",
  "enrich",
  "route",
  "buffer",
  "batch",
  "stream",
  "cache",
  "index",
  "archive",
  "notify",
  "log",
  "checkpoint",
  "snapshot",
  "replicate",
];

function CustomNode({ data }: NodeProps) {
  const d = data as { label: string; category: (typeof CATEGORIES)[0] };
  return (
    <div
      style={{
        background: d.category.bg,
        border: `2px solid ${d.category.color}`,
        borderRadius: 10,
        padding: "8px 16px",
        color: "#e2e8f0",
        fontSize: 12,
        fontFamily: "'Inter', system-ui, sans-serif",
        fontWeight: 500,
        minWidth: 140,
        textAlign: "center",
        boxShadow: `0 0 12px ${d.category.color}33, 0 4px 12px rgba(0,0,0,0.4)`,
      }}
    >
      <Handle
        type="target"
        position={Position.Top}
        style={{
          background: d.category.color,
          width: 8,
          height: 8,
          border: "none",
        }}
      />
      <div
        style={{
          color: d.category.color,
          fontSize: 9,
          textTransform: "uppercase",
          letterSpacing: 1.5,
          marginBottom: 3,
        }}
      >
        {d.category.name}
      </div>
      <div>{d.label}</div>
      <Handle
        type="source"
        position={Position.Bottom}
        style={{
          background: d.category.color,
          width: 8,
          height: 8,
          border: "none",
        }}
      />
    </div>
  );
}

const nodeTypes = { custom: CustomNode };

const NODE_W = 160;
const NODE_H = 50;
const GAP_X = 50;
const GAP_Y = 80;

function generateDAG(nodeCount: number): { nodes: Node[]; edges: Edge[] } {
  const nodes: Node[] = [];
  const edges: Edge[] = [];
  const edgeSet = new Set<string>();

  const layers: number[][] = [];
  let remaining = nodeCount;
  let id = 0;

  while (remaining > 0) {
    const layerSize = Math.min(
      Math.max(1, Math.floor(Math.random() * 8) + 2),
      remaining,
    );
    const layer: number[] = [];
    for (let j = 0; j < layerSize; j++) {
      const catIdx = Math.min(
        layers.length % CATEGORIES.length,
        CATEGORIES.length - 1,
      );
      const cat =
        CATEGORIES[
          (catIdx + Math.floor(Math.random() * 2)) % CATEGORIES.length
        ];
      const taskName = TASK_NAMES[id % TASK_NAMES.length];

      // Position directly from layer structure
      const layerWidth = layerSize * NODE_W + (layerSize - 1) * GAP_X;
      const x = j * (NODE_W + GAP_X) - layerWidth / 2;
      const y = layers.length * (NODE_H + GAP_Y);

      nodes.push({
        id: `${id}`,
        type: "custom",
        data: {
          label: `${taskName}_${id}`,
          category: cat,
        },
        position: { x, y },
      });
      layer.push(id);
      id++;
    }
    layers.push(layer);
    remaining -= layerSize;
  }

  // Connect layers
  for (let l = 0; l < layers.length - 1; l++) {
    for (const srcId of layers[l]) {
      const reach = Math.min(
        l + 1 + Math.floor(Math.random() * 2),
        layers.length - 1,
      );
      for (let tl = l + 1; tl <= reach; tl++) {
        const targetLayer = layers[tl];
        const numTargets = Math.min(
          1 + Math.floor(Math.random() * 2),
          targetLayer.length,
        );
        const shuffled = [...targetLayer].sort(() => Math.random() - 0.5);
        for (let t = 0; t < numTargets; t++) {
          const tgtId = shuffled[t];
          const edgeId = `e${srcId}-${tgtId}`;
          if (!edgeSet.has(edgeId)) {
            edgeSet.add(edgeId);
            const srcCat = (
              nodes[srcId].data as { category: (typeof CATEGORIES)[0] }
            ).category;
            edges.push({
              id: edgeId,
              source: `${srcId}`,
              target: `${tgtId}`,
              animated: Math.random() > 0.7,
              style: { stroke: srcCat.color, strokeWidth: 1.5, opacity: 0.6 },
              markerEnd: { type: MarkerType.ArrowClosed, color: srcCat.color },
            });
          }
        }
      }
    }
  }

  return { nodes, edges };
}

const NODE_COUNT = 2000;

const initialDAG = generateDAG(NODE_COUNT);

export default function App() {
  const [nodes, , onNodesChange] = useNodesState<Node>(initialDAG.nodes);
  const [edges, , onEdgesChange] = useEdgesState<Edge>(initialDAG.edges);

  return (
    <div style={{ width: "100vw", height: "100vh", background: "#0f0f1a" }}>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        nodeTypes={nodeTypes}
        fitView
        colorMode="dark"
        defaultEdgeOptions={{ animated: false }}
        minZoom={0.05}
        maxZoom={2}
      >
        <Background
          variant={BackgroundVariant.Dots}
          gap={20}
          size={1}
          color="#1e1e3a"
        />
        <Controls
          style={{
            background: "#1a1a2e",
            borderRadius: 8,
            border: "1px solid #2d2d4a",
          }}
        />
        <MiniMap
          nodeColor={(n) => {
            const cat = (n.data as { category: (typeof CATEGORIES)[0] })
              ?.category;
            return cat?.color ?? "#6366f1";
          }}
          maskColor="rgba(0, 0, 0, 0.7)"
          style={{
            background: "#0f0f1a",
            border: "1px solid #2d2d4a",
            borderRadius: 8,
          }}
        />
      </ReactFlow>
    </div>
  );
}
