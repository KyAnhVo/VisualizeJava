import { MarkerType, type Edge } from '@xyflow/react'
import {
  SOURCE_BOTTOM,
  SOURCE_TOP,
  TARGET_BOTTOM,
  TARGET_TOP,
  TYPE_NODE_TYPE,
  type TypeFlowNode,
} from '@/components/TypeNode'
import { NODE_HEIGHT, nodeWidth } from './node-size'
import {
  EDGE_COLOR,
  EDGE_COLOR_DIMMED,
  EDGE_COLOR_HIGHLIGHT,
  RELATION_STYLES,
  packageColor,
  type PackageColors,
} from './palette'
import type { ProjectGraph, ProjectRelation, RelationKind } from '@/types/graph'
import type { LayoutEdge, LayoutNode } from './layout'

export interface ViewState {
  visibleKinds: Set<RelationKind>
  hideUnconnected: boolean
  /** Node whose neighbourhood is isolated, or `null`. */
  isolatedId: string | null
  /** Ids matching the search term, or `null` when no search is active. */
  searchMatches: Set<string> | null
  selectedId: string | null
}

/** The types and relations a given view actually shows. */
export interface VisibleGraph {
  types: ProjectGraph['types']
  relations: ProjectRelation[]
}

/**
 * Applies the structural filters — the ones that change *which* nodes exist
 * and therefore require a fresh layout. Isolation and search only restyle, so
 * they are deliberately not part of this.
 */
export function selectVisible(
  graph: ProjectGraph,
  visibleKinds: Set<RelationKind>,
  hideUnconnected: boolean,
): VisibleGraph {
  const relations = graph.relations.filter((r) => visibleKinds.has(r.kind))
  if (!hideUnconnected) return { types: graph.types, relations }

  const connected = new Set<string>()
  for (const relation of relations) {
    connected.add(relation.source)
    connected.add(relation.target)
  }
  return {
    types: graph.types.filter((type) => connected.has(type.id)),
    relations,
  }
}

/** ELK input. Inheritance is reversed so supertypes land in earlier layers. */
export function toLayoutInput(visible: VisibleGraph): {
  nodes: LayoutNode[]
  edges: LayoutEdge[]
} {
  return {
    nodes: visible.types.map((type) => ({
      id: type.id,
      width: nodeWidth(type),
      height: NODE_HEIGHT,
    })),
    edges: visible.relations
      // A self-reference carries no layering information and makes ELK's
      // layered algorithm work harder for nothing.
      .filter((relation) => relation.source !== relation.target)
      .map((relation) =>
        relation.kind === 'association'
          ? {
              id: relation.id,
              source: relation.source,
              target: relation.target,
            }
          : {
              id: relation.id,
              source: relation.target,
              target: relation.source,
            },
      ),
  }
}

/**
 * The neighbourhood kept bright during isolation: the focused type plus every
 * type one hop away along a *currently visible* relation.
 */
export function neighborhoodOf(
  id: string,
  relations: ProjectRelation[],
): Set<string> {
  const neighborhood = new Set<string>([id])
  for (const relation of relations) {
    if (relation.source === id) neighborhood.add(relation.target)
    if (relation.target === id) neighborhood.add(relation.source)
  }
  return neighborhood
}

export function toFlowNodes(
  visible: VisibleGraph,
  positions: Map<string, { x: number; y: number }>,
  colors: PackageColors,
  view: ViewState,
): TypeFlowNode[] {
  const neighborhood = view.isolatedId
    ? neighborhoodOf(view.isolatedId, visible.relations)
    : null

  return visible.types.map((type) => {
    const outsideFocus = neighborhood ? !neighborhood.has(type.id) : false
    const unmatched = view.searchMatches
      ? !view.searchMatches.has(type.id)
      : false

    return {
      id: type.id,
      type: TYPE_NODE_TYPE,
      position: positions.get(type.id) ?? { x: 0, y: 0 },
      selected: view.selectedId === type.id,
      // Sizing React Flow's wrapper as well as the inner box keeps handle
      // anchoring in step with what ELK was told.
      width: nodeWidth(type),
      height: NODE_HEIGHT,
      data: {
        type,
        color: packageColor(colors, type.packageName),
        width: nodeWidth(type),
        dimmed: outsideFocus || unmatched,
        highlighted: view.searchMatches?.has(type.id) ?? false,
      },
    }
  })
}

export function toFlowEdges(
  visible: VisibleGraph,
  view: ViewState,
): Edge[] {
  const neighborhood = view.isolatedId
    ? neighborhoodOf(view.isolatedId, visible.relations)
    : null

  return visible.relations.map((relation) => {
    const style = RELATION_STYLES[relation.kind]
    const touchesFocus =
      view.isolatedId !== null &&
      (relation.source === view.isolatedId || relation.target === view.isolatedId)
    const dimmed = neighborhood
      ? !(neighborhood.has(relation.source) && neighborhood.has(relation.target))
      : false

    const color = touchesFocus
      ? EDGE_COLOR_HIGHLIGHT
      : dimmed
        ? EDGE_COLOR_DIMMED
        : EDGE_COLOR

    const inheritance = relation.kind !== 'association'

    return {
      id: relation.id,
      source: relation.source,
      target: relation.target,
      sourceHandle: inheritance ? SOURCE_TOP : SOURCE_BOTTOM,
      targetHandle: inheritance ? TARGET_BOTTOM : TARGET_TOP,
      // The members behind an association are shown only for the focused
      // node's own edges. Labelling every association would bury the graph:
      // a mid-sized project has an order of magnitude more of them than
      // inheritance edges.
      label:
        touchesFocus && relation.via.length
          ? relation.via.join(', ')
          : undefined,
      labelShowBg: true,
      labelBgPadding: [4, 2] as [number, number],
      labelBgBorderRadius: 3,
      labelStyle: { fill: 'oklch(0.708 0 0)', fontSize: 10 },
      labelBgStyle: { fill: 'oklch(0.205 0 0)', fillOpacity: 0.9 },
      style: {
        stroke: color,
        strokeWidth: touchesFocus ? style.width + 0.5 : style.width,
        strokeDasharray: style.dash,
        opacity: dimmed ? 0.35 : 1,
      },
      markerEnd: {
        type: style.closedArrow ? MarkerType.ArrowClosed : MarkerType.Arrow,
        color,
        width: 16,
        height: 16,
      },
      zIndex: touchesFocus ? 2 : 1,
    }
  })
}
