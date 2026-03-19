'use client';

import { useCallback, useEffect, useMemo } from 'react';
import {
  ReactFlow,
  Background,
  BackgroundVariant,
  Controls,
  MiniMap,
  useNodesState,
  useEdgesState,
  addEdge,
  MarkerType,
  type Node,
  type Edge,
  type Connection,
  Panel,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import WalletNode from './WalletNode';
import type { TraceData } from './types';

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const NODE_TYPES: Record<string, any> = { wallet: WalletNode };

const EDGE_STYLE_OUT = {
  stroke: 'rgba(203, 166, 247, 0.55)',
  strokeWidth: 1.5,
};
const EDGE_STYLE_IN = {
  stroke: 'rgba(148, 226, 213, 0.55)',
  strokeWidth: 1.5,
};
const EDGE_STYLE_BOTH = {
  stroke: 'rgba(137, 180, 250, 0.45)',
  strokeWidth: 1.5,
  strokeDasharray: '4 3',
};

function edgeStyle(dir: string) {
  if (dir === 'out')  return EDGE_STYLE_OUT;
  if (dir === 'in')   return EDGE_STYLE_IN;
  return EDGE_STYLE_BOTH;
}

function formatLamports(l: number) {
  if (l === 0) return '';
  const sol = l / 1e9;
  if (sol >= 1000) return `${(sol / 1000).toFixed(1)}K◎`;
  if (sol >= 1)    return `${sol.toFixed(2)}◎`;
  return `${Math.round(l / 1000)}K◁`;
}

/** Arrange nodes in a radial layout around the root */
function buildLayout(data: TraceData): { nodes: Node[]; edges: Edge[] } {
  const rootAddr = data.root;
  const peers = data.nodes.filter(n => n.address !== rootAddr);
  const cx = 0, cy = 0;

  const flowNodes: Node[] = [
    {
      id: rootAddr,
      type: 'wallet',
      position: { x: cx, y: cy },
      data: { ...data.nodes.find(n => n.address === rootAddr)!, isRoot: true },
    },
  ];

  // Radial: spread peers evenly, separate out vs in to left/right
  const outPeers = peers.filter((_, i) =>
    data.edges.find(e => e.source === rootAddr && e.target === peers[i]?.address)
  );
  const inPeers  = peers.filter(p => !outPeers.includes(p));

  const placeGroup = (group: typeof peers, side: 1 | -1, startY: number) => {
    const spacing = 160;
    const xOff = side * 420;
    group.forEach((node, i) => {
      const y = startY + i * spacing - ((group.length - 1) * spacing) / 2;
      flowNodes.push({
        id: node.address,
        type: 'wallet',
        position: { x: cx + xOff, y },
        data: { ...node, isRoot: false },
      });
    });
  };

  placeGroup(outPeers, 1,  0);
  placeGroup(inPeers,  -1, 0);

  const flowEdges: Edge[] = data.edges.map(e => ({
    id: e.id,
    source: e.source,
    target: e.target,
    type: 'smoothstep',
    animated: e.direction === 'out',
    style: edgeStyle(e.direction),
    label: e.tx_count > 1
      ? `${e.tx_count} txns  ${formatLamports(e.total_lamports)}`
      : formatLamports(e.total_lamports),
    labelStyle: { fill: '#6c7086', fontSize: 9, fontFamily: 'monospace' },
    labelBgStyle: { fill: 'rgba(0,0,0,0.7)', rx: 3 },
    markerEnd: {
      type: MarkerType.ArrowClosed,
      width: 10,
      height: 10,
      color: e.direction === 'out' ? '#cba6f7' : '#94e2d5',
    },
  }));

  return { nodes: flowNodes, edges: flowEdges };
}

interface Props {
  data: TraceData;
  onNodeClick?: (address: string) => void;
}

export default function TraceGraph({ data, onNodeClick }: Props) {
  const { nodes: initNodes, edges: initEdges } = useMemo(() => buildLayout(data), [data]);

  const [nodes, setNodes, onNodesChange] = useNodesState(initNodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(initEdges);

  useEffect(() => {
    const { nodes: n, edges: e } = buildLayout(data);
    setNodes(n);
    setEdges(e);
  }, [data, setNodes, setEdges]);

  const onConnect = useCallback(
    (connection: Connection) => setEdges(eds => addEdge(connection, eds)),
    [setEdges]
  );

  const onNodeClickCb = useCallback(
    (_: React.MouseEvent, node: Node) => onNodeClick?.(node.id),
    [onNodeClick]
  );

  return (
    <div style={{ width: '100%', height: '100%', background: '#050508' }}>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onConnect={onConnect}
        onNodeClick={onNodeClickCb}
        nodeTypes={NODE_TYPES}
        fitView
        fitViewOptions={{ padding: 0.18, maxZoom: 1.2 }}
        minZoom={0.1}
        maxZoom={3}
        proOptions={{ hideAttribution: true }}
        style={{ background: 'transparent' }}
      >
        <Background
          variant={BackgroundVariant.Dots}
          gap={28}
          size={1}
          color="rgba(255,255,255,0.04)"
        />

        <Controls
          style={{
            background: 'rgba(10,15,20,0.9)',
            border: '1px solid rgba(255,255,255,0.1)',
            borderRadius: 6,
          }}
        />

        <MiniMap
          style={{
            background: 'rgba(10,15,20,0.9)',
            border: '1px solid rgba(255,255,255,0.1)',
          }}
          nodeColor={(n) => (n.data as { isRoot?: boolean }).isRoot ? '#00e5ff' : '#313244'}
          maskColor="rgba(0,0,0,0.7)"
        />

        {/* Legend */}
        <Panel position="top-left">
          <div
            style={{
              background: 'rgba(10,15,20,0.85)',
              border: '1px solid rgba(255,255,255,0.08)',
              borderRadius: 6,
              padding: '8px 12px',
              fontFamily: 'monospace',
              fontSize: 10,
              color: '#6c7086',
              display: 'flex',
              flexDirection: 'column',
              gap: 5,
            }}
          >
            <LegendRow color="#cba6f7" label="Outflow" dashed={false} />
            <LegendRow color="#94e2d5" label="Inflow"  dashed={false} />
            <LegendRow color="#89b4fa" label="Bidirectional" dashed />
          </div>
        </Panel>
      </ReactFlow>
    </div>
  );
}

function LegendRow({ color, label, dashed }: { color: string; label: string; dashed: boolean }) {
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
      <svg width={28} height={8}>
        <line
          x1={0} y1={4} x2={28} y2={4}
          stroke={color}
          strokeWidth={2}
          strokeDasharray={dashed ? '4 2' : undefined}
        />
      </svg>
      <span style={{ color: '#a6adc8' }}>{label}</span>
    </div>
  );
}
