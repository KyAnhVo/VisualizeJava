import { useCallback, useEffect, useMemo, useRef } from 'react'
import {
  Background,
  BackgroundVariant,
  Controls,
  MiniMap,
  ReactFlow,
  ReactFlowProvider,
  useReactFlow,
  type Edge,
  type NodeMouseHandler,
} from '@xyflow/react'
import { TypeNode, type TypeFlowNode } from '@/components/TypeNode'
import { NEUTRAL_PACKAGE_COLOR } from '@/lib/palette'

const nodeTypes = { javaType: TypeNode }

export interface FocusRequest {
  ids: string[]
  /** Changes on every request so repeat focus on the same node still fires. */
  nonce: number
}

interface GraphCanvasProps {
  nodes: TypeFlowNode[]
  edges: Edge[]
  focus: FocusRequest | null
  /**
   * Bumped whenever a fresh layout lands. Positions arrive asynchronously,
   * well after React Flow's one-shot `fitView` prop has already run against an
   * unpositioned graph, so the fit has to be re-triggered explicitly.
   */
  fitSignal: number
  onSelect: (id: string | null) => void
  /** Shown over the canvas while a layout is in flight. */
  busy: boolean
  overlay?: React.ReactNode
}

function Canvas({
  nodes,
  edges,
  focus,
  fitSignal,
  onSelect,
  busy,
  overlay,
}: GraphCanvasProps) {
  const { fitView } = useReactFlow()
  const lastNonce = useRef<number | null>(null)
  // Re-fitting on every render would fight the user's panning; only a fresh
  // focus request (new nonce) moves the viewport.
  const nodeIds = useMemo(() => new Set(nodes.map((node) => node.id)), [nodes])

  useEffect(() => {
    if (fitSignal === 0 || nodes.length === 0) return
    // One frame of slack so React Flow has measured the newly placed nodes.
    const frame = requestAnimationFrame(() => {
      void fitView({ duration: 350, padding: 0.12 })
    })
    return () => cancelAnimationFrame(frame)
    // Intentionally keyed on the signal alone: node identity changes on every
    // restyle (dimming, selection), which must not move the viewport.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [fitSignal])

  useEffect(() => {
    if (!focus || focus.nonce === lastNonce.current) return
    lastNonce.current = focus.nonce
    const present = focus.ids.filter((id) => nodeIds.has(id))
    if (present.length === 0) return
    void fitView({
      nodes: present.map((id) => ({ id })),
      duration: 400,
      // Zooming to a single node fills the screen with it; cap it.
      maxZoom: present.length === 1 ? 1.4 : 1,
      padding: 0.35,
    })
  }, [fitView, focus, nodeIds])

  const handleNodeClick = useCallback<NodeMouseHandler>(
    (_event, node) => onSelect(node.id),
    [onSelect],
  )

  return (
    <div className="relative h-full w-full">
      <ReactFlow
        nodes={nodes}
        edges={edges}
        nodeTypes={nodeTypes}
        onNodeClick={handleNodeClick}
        onPaneClick={() => onSelect(null)}
        // The graph is a read-only view of parsed source.
        nodesConnectable={false}
        edgesFocusable={false}
        nodesDraggable
        elevateNodesOnSelect
        minZoom={0.05}
        maxZoom={2.5}
        fitView
        proOptions={{ hideAttribution: false }}
      >
        <Background
          variant={BackgroundVariant.Dots}
          gap={22}
          size={1}
          color="oklch(0.30 0 0)"
        />
        <Controls showInteractive={false} position="bottom-right" />
        <MiniMap
          pannable
          zoomable
          position="top-right"
          maskColor="oklch(0.145 0 0 / 78%)"
          nodeColor={(node) =>
            (node.data as { color?: string }).color ?? NEUTRAL_PACKAGE_COLOR
          }
          nodeStrokeWidth={0}
          className="!rounded-md !border !border-border"
        />
      </ReactFlow>

      <div className="pointer-events-none absolute bottom-3 left-3 z-10 max-w-56">
        {overlay}
      </div>

      {busy && (
        <div className="absolute inset-x-0 top-0 z-10 flex justify-center pt-3">
          <span className="rounded-full border border-border bg-card px-3 py-1 text-xs text-muted-foreground">
            Laying out…
          </span>
        </div>
      )}
    </div>
  )
}

export function GraphCanvas(props: GraphCanvasProps) {
  return (
    <ReactFlowProvider>
      <Canvas {...props} />
    </ReactFlowProvider>
  )
}
