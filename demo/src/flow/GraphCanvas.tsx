import {
  Background,
  BackgroundVariant,
  Controls,
  MarkerType,
  MiniMap,
  ReactFlow,
  ReactFlowProvider,
  useReactFlow,
  useStore,
  type Edge,
  type Node,
  type NodeTypes,
} from '@xyflow/react'
import { useCallback, useEffect, useMemo, useState } from 'react'

import { Breadcrumb } from '@/components/Breadcrumb'
import { Inspector } from '@/components/Inspector'
import { Legend } from '@/components/Legend'
import { TypeSearch } from '@/components/TypeSearch'
import {
  assocHandles,
  HANDLE_INHERIT_SOURCE,
  HANDLE_INHERIT_TARGET,
} from '@/flow/handles'
import {
  layoutGraph,
  layoutPackages,
  type LayoutResult,
  type PackageLayoutResult,
} from '@/flow/layout'
import { PackageNode, type PackageNodeData } from '@/flow/PackageNode'
import { TypeNode, type TypeNodeData } from '@/flow/TypeNode'
import type { GraphModel } from '@/model/build'
import { buildPackageGraph } from '@/model/packages'
import {
  focusSubgraph,
  hiddenNeighbourCounts,
  packageSubgraph,
} from '@/model/subgraph'
import { HOME, levelKey, type Level } from '@/state/level'
import {
  deriveHighlight,
  isSameSelection,
  NO_SELECTION,
  type Selection,
} from '@/state/selection'

const TYPE_NODE = 'type'
const PACKAGE_NODE = 'package'
const nodeTypes: NodeTypes = { [TYPE_NODE]: TypeNode, [PACKAGE_NODE]: PackageNode }

const KIND_COLOR = {
  class: 'var(--kind-class)',
  interface: 'var(--kind-interface)',
  enum: 'var(--kind-enum)',
  annotation: 'var(--kind-annotation)',
} as const

/**
 * Level-of-detail thresholds.
 *
 * Only the `all` escape hatch and the largest packages still reach a zoom where
 * text stops resolving, but the tiers cost nothing to keep: the node sheds
 * detail as it shrinks — full card, then the name alone, then a solid block in
 * the type's colour so the inheritance skeleton is what reads at an overview.
 */
type ZoomTier = 'near' | 'mid' | 'far'
function zoomTier(zoom: number): ZoomTier {
  if (zoom >= 0.5) return 'near'
  if (zoom >= 0.28) return 'mid'
  return 'far'
}

/** A laid-out level, tagged with the level it was computed for. */
type Stage =
  | { key: string; kind: 'packages'; layout: PackageLayoutResult }
  | { key: string; kind: 'types'; view: GraphModel; layout: LayoutResult }

export function GraphCanvas({ model }: { model: GraphModel }) {
  const [level, setLevel] = useState<Level>(HOME)
  const [history, setHistory] = useState<Level[]>([])
  const [selection, setSelection] = useState<Selection>(NO_SELECTION)
  const [stage, setStage] = useState<Stage | null>(null)
  const { fitView } = useReactFlow()

  // Selecting on a primitive keeps this from re-rendering on every zoom frame —
  // only a tier change matters.
  const tier = useStore((state) => zoomTier(state.transform[2]))

  const packageGraph = useMemo(() => buildPackageGraph(model), [model])
  const key = levelKey(level)

  // A new graph invalidates every key we were holding on to. Resetting during
  // render is React's documented way to drop state a changed prop invalidates;
  // an effect would render once with the previous graph's selection still on.
  const [graph, setGraph] = useState(model)
  if (graph !== model) {
    setGraph(model)
    setLevel(HOME)
    setHistory([])
    setSelection(NO_SELECTION)
    setStage(null)
  }

  useEffect(() => {
    let cancelled = false

    const run = async (): Promise<Stage> => {
      if (level.kind === 'packages') {
        return { key, kind: 'packages', layout: await layoutPackages(packageGraph) }
      }
      const view =
        level.kind === 'all'
          ? model
          : level.kind === 'package'
            ? packageSubgraph(model, level.name)
            : focusSubgraph(model, level.typeKey)
      return { key, kind: 'types', view, layout: await layoutGraph(view) }
    }

    void run().then((next) => {
      if (!cancelled) setStage(next)
    })
    return () => {
      cancelled = true
    }
    // `key` is the value-identity of `level`.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [model, packageGraph, key])

  const ready = stage?.key === key

  // Each level is a different graph, so frame it rather than leaving the
  // viewport wherever the previous one left it.
  useEffect(() => {
    if (!ready) return
    const frame = requestAnimationFrame(() => {
      void fitView({ padding: 0.16, maxZoom: 1, duration: 400 })
    })
    return () => cancelAnimationFrame(frame)
  }, [ready, key, fitView])

  const goTo = useCallback((next: Level) => {
    setSelection(NO_SELECTION)
    setLevel((current) => {
      if (levelKey(current) === levelKey(next)) return current
      setHistory((stack) => [...stack, current])
      return next
    })
  }, [])

  const goBack = useCallback(() => {
    setSelection(NO_SELECTION)
    setHistory((stack) => {
      if (stack.length === 0) return stack
      setLevel(stack[stack.length - 1])
      return stack.slice(0, -1)
    })
  }, [])

  const goHome = useCallback(() => goTo(HOME), [goTo])

  const openPackage = useCallback(
    (name: string) => goTo({ kind: 'package', name }),
    [goTo],
  )

  /** Search, the navigator and boundary nodes all mean "go read that type". */
  const focusType = useCallback(
    (typeKey: string) => goTo({ kind: 'focus', typeKey }),
    [goTo],
  )

  // Highlighting is derived from the *full* model, not the view: the inspector
  // must report every type that refers to this one, including ones this level
  // does not draw — that is how the user finds where to navigate next. Edges
  // whose other end is off-view are simply skipped below.
  const highlight = useMemo(
    () => deriveHighlight(selection, model),
    [selection, model],
  )

  const selectType = useCallback((typeKey: string) => {
    setSelection((current) =>
      isSameSelection(current, { kind: 'type', typeKey })
        ? NO_SELECTION
        : { kind: 'type', typeKey },
    )
  }, [])

  const selectMember = useCallback((typeKey: string, memberKey: string) => {
    setSelection((current) =>
      isSameSelection(current, { kind: 'member', typeKey, memberKey })
        ? NO_SELECTION
        : { kind: 'member', typeKey, memberKey },
    )
  }, [])

  const reset = useCallback(() => setSelection(NO_SELECTION), [])

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') return
      // Escape unwinds one step at a time: clear the selection, then the level.
      if (selection.kind !== 'none') reset()
      else goBack()
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [reset, goBack, selection.kind])

  const hidden = useMemo(
    () =>
      stage?.kind === 'types' && level.kind === 'focus'
        ? hiddenNeighbourCounts(model, stage.view)
        : new Map<string, number>(),
    [stage, model, level.kind],
  )

  const nodes = useMemo<Node[]>(() => {
    if (!stage) return []

    if (stage.kind === 'packages') {
      const result: Node[] = []
      for (const laid of stage.layout.values()) {
        const pkg = packageGraph.packages.get(laid.key)
        if (!pkg) continue
        result.push({
          id: laid.key,
          type: PACKAGE_NODE,
          position: { x: laid.x, y: laid.y },
          width: laid.width,
          height: laid.height,
          style: { width: laid.width, height: laid.height },
          data: {
            pkg,
            geometry: laid.geometry,
            dimmed: false,
            onOpen: openPackage,
          } satisfies PackageNodeData,
        })
      }
      return result
    }

    const result: Node[] = []
    for (const laid of stage.layout.values()) {
      const type = stage.view.types.get(laid.key)
      if (!type) continue

      result.push({
        id: laid.key,
        type: TYPE_NODE,
        position: { x: laid.x, y: laid.y },
        parentId: laid.parentKey ?? undefined,
        extent: laid.parentKey ? 'parent' : undefined,
        width: laid.width,
        height: laid.height,
        style: { width: laid.width, height: laid.height },
        data: {
          type,
          geometry: laid.geometry,
          dimmed: highlight.active && !highlight.litTypes.has(laid.key),
          selected: highlight.anchorKey === laid.key,
          linked:
            highlight.active &&
            highlight.anchorKey !== laid.key &&
            highlight.litTypes.has(laid.key),
          focused: level.kind === 'focus' && level.typeKey === laid.key,
          hiddenNeighbours: hidden.get(laid.key) ?? 0,
          onSelectType: selectType,
          onExpand: focusType,
        } satisfies TypeNodeData,
      })
    }
    return result
  }, [
    stage,
    packageGraph,
    highlight,
    hidden,
    level,
    openPackage,
    selectType,
    focusType,
  ])

  const edges = useMemo<Edge[]>(() => {
    if (!stage) return []

    if (stage.kind === 'packages') {
      return packageGraph.edges
        .filter(
          (edge) => stage.layout.has(edge.source) && stage.layout.has(edge.target),
        )
        .map((edge) => {
          // Inheritance is rare between packages — 5 of 37 edges in `prod` — so
          // when it is present it is the more interesting fact and wins the
          // styling.
          const structural = edge.inheritance > 0
          const color = structural
            ? 'var(--edge-extends)'
            : 'var(--edge-assoc)'
          return {
            id: edge.id,
            source: edge.source,
            target: edge.target,
            sourceHandle: HANDLE_INHERIT_SOURCE,
            targetHandle: HANDLE_INHERIT_TARGET,
            type: 'smoothstep',
            style: {
              stroke: color,
              // Thickness is how many distinct type pairs are behind the edge.
              strokeWidth: Math.min(6, 1.4 + Math.log2(edge.weight)),
              opacity: structural ? 1 : 0.75,
            },
            label: edge.weight > 1 ? String(edge.weight) : undefined,
            labelShowBg: true,
            labelStyle: { fill: color, fontSize: 10, fontFamily: 'var(--font-mono)' },
            labelBgStyle: { fill: 'var(--card)', fillOpacity: 0.9 },
            labelBgPadding: [4, 2] as [number, number],
            labelBgBorderRadius: 3,
            markerEnd: {
              type: MarkerType.ArrowClosed,
              color,
              width: 18,
              height: 18,
            },
          } satisfies Edge
        })
    }

    const { view, layout } = stage
    const result: Edge[] = []

    for (const edge of view.inheritance) {
      if (!layout.has(edge.source) || !layout.has(edge.target)) continue
      const isExtends = edge.kind === 'Extends'
      const color = isExtends ? 'var(--edge-extends)' : 'var(--edge-implements)'
      // An inheritance edge stays lit only when both of its ends are relevant to
      // the current selection; otherwise it is background noise.
      const dimmed =
        highlight.active &&
        !(
          highlight.litTypes.has(edge.source) && highlight.litTypes.has(edge.target)
        )

      result.push({
        id: edge.id,
        source: edge.source,
        target: edge.target,
        sourceHandle: HANDLE_INHERIT_SOURCE,
        targetHandle: HANDLE_INHERIT_TARGET,
        type: 'smoothstep',
        className: dimmed ? 'vj-fade vj-dim' : 'vj-fade',
        style: {
          stroke: color,
          strokeWidth: isExtends ? 3.4 : 1.8,
          strokeDasharray: isExtends ? undefined : '7 5',
        },
        markerEnd: {
          type: isExtends ? MarkerType.ArrowClosed : MarkerType.Arrow,
          color,
          width: isExtends ? 22 : 18,
          height: isExtends ? 22 : 18,
          strokeWidth: isExtends ? 1 : 1.6,
        },
        zIndex: isExtends ? 3 : 2,
      })
    }

    for (const link of highlight.links) {
      const owner = layout.get(link.ownerKey)
      const target = layout.get(link.targetKey)
      // Links to types this level does not draw are dropped here; the inspector
      // still lists them, and clicking one navigates to that type.
      if (!owner || !target) continue
      // A self-reference has nowhere to go; the inspector's member list already
      // reports it, and a loop back into the same box only adds clutter.
      if (link.ownerKey === link.targetKey) continue

      // Leave from whichever side faces the target so the edge does not cut back
      // across its own node.
      const handles = assocHandles(target.absX >= owner.absX)
      const count = link.memberKeys.length
      const first = view.types
        .get(link.ownerKey)
        ?.members.find((m) => m.key === link.memberKeys[0])

      result.push({
        id: link.id,
        source: link.ownerKey,
        target: link.targetKey,
        sourceHandle: handles.source,
        targetHandle: handles.target,
        type: 'default',
        animated: true,
        className: 'vj-fade',
        style: { stroke: 'var(--edge-assoc)', strokeWidth: 2 },
        // The edge is aggregated to the type level, so it says how many members
        // are behind it; the inspector lists them.
        label: count > 1 ? `${count} members` : (first?.name ?? ''),
        labelShowBg: true,
        labelStyle: { fill: 'var(--edge-assoc)', fontSize: 10, fontFamily: 'var(--font-mono)' },
        labelBgStyle: { fill: 'var(--card)', fillOpacity: 0.9 },
        labelBgPadding: [4, 2],
        labelBgBorderRadius: 3,
        markerEnd: {
          type: MarkerType.ArrowClosed,
          color: 'var(--edge-assoc)',
          width: 18,
          height: 18,
        },
        zIndex: 2000,
      })
    }

    return result
  }, [stage, packageGraph, highlight])

  return (
    <div className="relative h-full w-full">
      <ReactFlow
        className={`vj-zoom-${tier}`}
        nodes={nodes}
        edges={edges}
        nodeTypes={nodeTypes}
        onPaneClick={reset}
        nodesConnectable={false}
        elementsSelectable={false}
        proOptions={{ hideAttribution: true }}
        minZoom={0.02}
        maxZoom={2.5}
        defaultEdgeOptions={{ interactionWidth: 12 }}
        style={{ background: 'var(--canvas)' }}
      >
        <Background
          variant={BackgroundVariant.Dots}
          gap={22}
          size={1.4}
          color="var(--canvas-dot)"
        />
        <Controls
          showInteractive={false}
          className="!border-border !bg-card !text-foreground [&_button]:!border-border [&_button]:!bg-card [&_button]:!fill-foreground [&_button:hover]:!bg-muted"
        />
        <MiniMap
          pannable
          zoomable
          className="!bg-card"
          maskColor="color-mix(in oklch, var(--background) 78%, transparent)"
          nodeColor={(node) => {
            const type = model.types.get(node.id)
            return type ? KIND_COLOR[type.kind] : 'var(--muted)'
          }}
          nodeStrokeWidth={0}
        />
      </ReactFlow>

      <div className="pointer-events-none absolute left-3 top-3 flex flex-col items-start gap-2">
        <div className="pointer-events-auto">
          <Breadcrumb
            level={level}
            model={model}
            depth={history.length}
            onHome={goHome}
            onBack={goBack}
          />
        </div>
        <Legend mode={level.kind === 'packages' ? 'packages' : 'types'} />
      </div>

      <TypeSearch model={model} onPick={focusType} />

      {selection.kind === 'none' ? (
        <div className="pointer-events-none absolute bottom-3 left-1/2 -translate-x-1/2 select-none rounded-full border border-border bg-card/85 px-4 py-1.5 text-[11px] text-muted-foreground backdrop-blur">
          {level.kind === 'packages'
            ? 'click a package to see its types'
            : 'click a type to open its members and see what uses it'}
        </div>
      ) : (
        <Inspector
          model={model}
          selection={selection}
          inView={stage?.kind === 'types' ? stage.view : null}
          onSelectMember={selectMember}
          onJump={focusType}
          onClose={reset}
        />
      )}

      {!ready && (
        <div className="absolute inset-0 grid place-items-center bg-background/60 text-sm text-muted-foreground">
          laying out…
        </div>
      )}
    </div>
  )
}

export function GraphCanvasProvider(props: { model: GraphModel }) {
  return (
    <ReactFlowProvider>
      <GraphCanvas {...props} />
    </ReactFlowProvider>
  )
}
