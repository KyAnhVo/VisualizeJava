import ELK, { type ElkNode } from 'elkjs/lib/elk.bundled.js'

import {
  CHILD_LABEL_H,
  CHILD_MIN_H,
  CHILD_MIN_W,
  CHILD_PAD,
  measurePackage,
  measureType,
  type PackageGeometry,
  type TypeGeometry,
} from '@/flow/geometry'
import type { GraphModel } from '@/model/build'
import type { PackageGraph } from '@/model/packages'

const elk = new ELK()

export interface PlacedNode {
  key: string
  parentKey: string | null
  /** Relative to the parent node, as React Flow expects for subflows. */
  x: number
  y: number
  /** Absolute canvas coordinates — used to pick which side an edge leaves from. */
  absX: number
  absY: number
  width: number
  height: number
}

export interface LaidOutNode extends PlacedNode {
  geometry: TypeGeometry
}
export type LayoutResult = Map<string, LaidOutNode>

export interface LaidOutPackage extends PlacedNode {
  geometry: PackageGeometry
}
export type PackageLayoutResult = Map<string, LaidOutPackage>

interface LayoutEdge {
  id: string
  source: string
  target: string
}

const COMPONENT_LAYOUT = {
  'elk.algorithm': 'layered',
  'elk.direction': 'UP',
  'elk.hierarchyHandling': 'INCLUDE_CHILDREN',
  'elk.spacing.nodeNode': '56',
  'elk.layered.spacing.nodeNodeBetweenLayers': '96',
  'elk.layered.spacing.edgeNodeBetweenLayers': '32',
  'elk.layered.nodePlacement.strategy': 'NETWORK_SIMPLEX',
}

/**
 * Lays the type graph out with ELK.
 *
 * Only inheritance edges take part. That is deliberate: with `direction: UP`,
 * ELK's layer index becomes inheritance depth — supertypes sit above their
 * subtypes — which is exactly the structure the demo is meant to make obvious.
 * Feeding association edges in as well would fight that ordering, and they are
 * hidden until the user selects something anyway.
 *
 * Inner types are ELK children of their outer type. The outer type's top
 * padding is its own rendered content, so ELK packs the nested types into the
 * space below it and grows the parent box to fit.
 *
 * The result depends only on the model: node sizes never change with selection,
 * so this runs once per view rather than on every click.
 */
export async function layoutGraph(model: GraphModel): Promise<LayoutResult> {
  const geometries = new Map<string, TypeGeometry>()
  for (const [key, type] of model.types) {
    geometries.set(key, measureType(type))
  }

  const build = (key: string): ElkNode => {
    const type = model.types.get(key)!
    const geometry = geometries.get(key)!

    if (!type.childKeys.length) {
      return { id: key, width: geometry.width, height: geometry.contentHeight }
    }

    const top = geometry.contentHeight + CHILD_LABEL_H + CHILD_PAD
    return {
      id: key,
      layoutOptions: {
        'elk.padding': `[top=${top},left=${CHILD_PAD},bottom=${CHILD_PAD},right=${CHILD_PAD}]`,
        'elk.nodeSize.constraints': 'MINIMUM_SIZE',
        'elk.nodeSize.minimum': `(${Math.max(geometry.width, CHILD_MIN_W + 2 * CHILD_PAD)},${top + CHILD_MIN_H + CHILD_PAD})`,
        'elk.algorithm': 'layered',
        'elk.direction': 'UP',
        'elk.spacing.nodeNode': '20',
        'elk.layered.spacing.nodeNodeBetweenLayers': '32',
      },
      children: type.childKeys.map(build),
    }
  }

  const placement = await place(model.rootKeys, build, model.inheritance, (key) =>
    outermost(key, model),
  )

  const result: LayoutResult = new Map()
  for (const [key, node] of placement) {
    result.set(key, { ...node, geometry: geometries.get(key)! })
  }
  return result
}

/**
 * Lays the package overview out.
 *
 * Both relations feed the layout here, not just inheritance: only 5 of `prod`'s
 * 37 package edges are inheritance, so an inheritance-only package graph would
 * be almost entirely disconnected boxes.
 */
export async function layoutPackages(
  graph: PackageGraph,
): Promise<PackageLayoutResult> {
  const geometries = new Map<string, PackageGeometry>()
  for (const [name, pkg] of graph.packages) {
    geometries.set(name, measurePackage(pkg))
  }

  const build = (key: string): ElkNode => {
    const geometry = geometries.get(key)!
    return { id: key, width: geometry.width, height: geometry.height }
  }

  const placement = await place(graph.order, build, graph.edges, (key) => key)

  const result: PackageLayoutResult = new Map()
  for (const [key, node] of placement) {
    result.set(key, { ...node, geometry: geometries.get(key)! })
  }
  return result
}

// --- generic placement -------------------------------------------------------

/**
 * Lays out one connected component at a time, then packs the results.
 *
 * `elk.separateConnectedComponents` is ignored under `hierarchyHandling:
 * INCLUDE_CHILDREN`, which nested types need. Handed the whole graph, ELK put
 * all 209 top-level types of `test_target/prod` into shared layers — one of them
 * holding every edgeless type — and returned a 41,000 × 4,000 strip with
 * unrelated components interleaved. Per-component layout keeps each block
 * compact and lets `packComponents` arrange them into a screen shape: the same
 * graph comes back 6,791 × 4,100.
 */
async function place(
  rootKeys: string[],
  build: (key: string) => ElkNode,
  edges: LayoutEdge[],
  outermostOf: (key: string) => string,
): Promise<Map<string, PlacedNode>> {
  const result = new Map<string, PlacedNode>()
  const components: Component[] = []

  for (const keys of componentsOf(rootKeys, edges, outermostOf)) {
    const nodes: PlacedNode[] = []
    const elkRoots = keys.map(build)

    // Most components are a single box with nothing nested in it. Running the
    // layered algorithm on those would be scores of calls to place one node at
    // the origin.
    if (elkRoots.length === 1 && !elkRoots[0].children?.length) {
      const only = elkRoots[0]
      nodes.push({
        key: only.id,
        parentKey: null,
        x: 0,
        y: 0,
        absX: 0,
        absY: 0,
        width: only.width ?? 0,
        height: only.height ?? 0,
      })
    } else {
      const members = new Set(keys)
      const laid = await elk.layout({
        id: `component:${keys[0]}`,
        layoutOptions: COMPONENT_LAYOUT,
        children: elkRoots,
        edges: edges
          .filter(
            (edge) =>
              members.has(outermostOf(edge.source)) &&
              members.has(outermostOf(edge.target)),
          )
          .map((edge) => ({
            id: edge.id,
            sources: [edge.source],
            targets: [edge.target],
          })),
      })
      collect(laid, null, 0, 0, nodes)
    }

    for (const node of nodes) result.set(node.key, node)

    const roots = nodes.filter((node) => !node.parentKey)
    // ELK does not guarantee the block starts at the origin; normalise so the
    // packer can treat a component's position as its top-left corner.
    const minX = Math.min(...roots.map((n) => n.x))
    const minY = Math.min(...roots.map((n) => n.y))
    for (const root of roots) translate(root, result, -minX, -minY)

    components.push({
      roots,
      width: Math.max(...roots.map((n) => n.x + n.width)),
      height: Math.max(...roots.map((n) => n.y + n.height)),
    })
  }

  packComponents(components, result)

  return result
}

interface Component {
  roots: PlacedNode[]
  width: number
  height: number
}

function collect(
  node: ElkNode,
  parentKey: string | null,
  originX: number,
  originY: number,
  into: PlacedNode[],
) {
  for (const child of node.children ?? []) {
    const x = child.x ?? 0
    const y = child.y ?? 0
    const absX = originX + x
    const absY = originY + y
    into.push({
      key: child.id,
      parentKey,
      x,
      y,
      absX,
      absY,
      width: child.width ?? 0,
      height: child.height ?? 0,
    })
    collect(child, child.id, absX, absY, into)
  }
}

/**
 * Root keys grouped by connectivity.
 *
 * An edge can touch a nested type; it travels with its outermost enclosing type,
 * because that is the box actually placed on the canvas.
 */
function componentsOf(
  rootKeys: string[],
  edges: LayoutEdge[],
  outermostOf: (key: string) => string,
): string[][] {
  const parentOf = new Map<string, string>()
  const find = (key: string): string => {
    let root = key
    while (parentOf.get(root) !== undefined && parentOf.get(root) !== root) {
      root = parentOf.get(root)!
    }
    return root
  }

  for (const key of rootKeys) parentOf.set(key, key)
  for (const edge of edges) {
    const a = find(outermostOf(edge.source))
    const b = find(outermostOf(edge.target))
    if (parentOf.has(a) && parentOf.has(b) && a !== b) parentOf.set(a, b)
  }

  const groups = new Map<string, string[]>()
  // `rootKeys` arrives in display order, so the grouping is deterministic.
  for (const key of rootKeys) {
    const root = find(key)
    groups.set(root, [...(groups.get(root) ?? []), key])
  }
  return [...groups.values()]
}

const COMPONENT_GAP = 72
/** Shaped for a widescreen viewport, since `fitView` is what has to swallow it. */
const TARGET_ASPECT = 1.78
/** Multiples of the area-derived estimate to try when picking a shelf width. */
const SHELF_TRIALS = [0.6, 0.75, 0.9, 1, 1.15, 1.35, 1.6, 2, 2.5]

/**
 * Shelf-packs the laid-out components into a screen-shaped block.
 *
 * In a real project most types have no supertype at all, so the components are
 * mostly single boxes; left in a row they would run off the canvas. Each
 * component's internal layout is untouched — only its origin moves.
 *
 * The shelf width is *chosen*, not computed: shelf packing wastes a variable
 * amount of space depending on how the component heights happen to fall, so a
 * single area-derived guess can be badly off (it packed the 17-type fixture into
 * a 0.63-aspect column). Packing is linear and there are at most a few hundred
 * components, so trying a spread of widths and keeping the one that lands
 * closest to `TARGET_ASPECT` costs nothing.
 */
function packComponents(components: Component[], layout: Map<string, PlacedNode>) {
  if (components.length < 2) return

  const ordered = [...components].sort(
    (a, b) => b.height - a.height || b.width - a.width,
  )

  // Gaps are most of the area when the components are single boxes, so they
  // belong in the estimate.
  const area = ordered.reduce(
    (sum, c) => sum + (c.width + COMPONENT_GAP) * (c.height + COMPONENT_GAP),
    0,
  )
  const base = Math.sqrt(area * TARGET_ASPECT)

  let best: Placement[] | null = null
  let bestScore = Infinity
  for (const factor of SHELF_TRIALS) {
    const trial = shelve(ordered, base * factor)
    // Compare in log space so 2x too wide and 2x too tall score the same.
    const score = Math.abs(
      Math.log(trial.width / trial.height) - Math.log(TARGET_ASPECT),
    )
    if (score < bestScore) {
      bestScore = score
      best = trial.placements
    }
  }

  ordered.forEach((component, index) => {
    const { x, y } = best![index]
    for (const root of component.roots) translate(root, layout, x, y)
  })
}

interface Placement {
  x: number
  y: number
}

function shelve(ordered: Component[], shelfWidth: number) {
  const placements: Placement[] = []
  let cursorX = 0
  let cursorY = 0
  let shelfHeight = 0
  let width = 0

  const newShelf = () => {
    cursorX = 0
    cursorY += shelfHeight + COMPONENT_GAP
    shelfHeight = 0
  }

  for (const component of ordered) {
    // A component wider than the shelf — one broad inheritance layer, say — gets
    // a shelf to itself rather than setting the width for all the others.
    const oversized = component.width > shelfWidth
    if (cursorX > 0 && (oversized || cursorX + component.width > shelfWidth)) {
      newShelf()
    }

    placements.push({ x: cursorX, y: cursorY })

    cursorX += component.width + COMPONENT_GAP
    width = Math.max(width, cursorX - COMPONENT_GAP)
    shelfHeight = Math.max(shelfHeight, component.height)
    if (oversized) newShelf()
  }

  return { placements, width, height: cursorY + shelfHeight }
}

/**
 * Moves a root box. Only the root's own `x`/`y` change — a nested node's
 * position is relative to its parent — but every descendant's absolute
 * coordinates do, and those decide which side an association edge leaves from.
 */
function translate(
  node: PlacedNode,
  layout: Map<string, PlacedNode>,
  dx: number,
  dy: number,
) {
  node.x += dx
  node.y += dy
  shiftAbsolute(node, layout, dx, dy)
}

function shiftAbsolute(
  node: PlacedNode,
  layout: Map<string, PlacedNode>,
  dx: number,
  dy: number,
) {
  node.absX += dx
  node.absY += dy
  for (const other of layout.values()) {
    if (other.parentKey === node.key) shiftAbsolute(other, layout, dx, dy)
  }
}

function outermost(key: string, model: GraphModel): string {
  let cursor = key
  for (let guard = 0; guard < 16; guard += 1) {
    const parent = model.types.get(cursor)?.parentKey
    if (!parent) return cursor
    cursor = parent
  }
  return cursor
}
