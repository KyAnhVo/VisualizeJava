import ELK from 'elkjs/lib/elk-api.js'
// `?worker` makes Vite build this file as a worker entry and hand back a
// constructor. That is the only way to load ELK's engine: the file detects a
// worker context and installs itself as the worker's message handler, so it
// must *be* a worker rather than be imported into one.
import ElkEngineWorker from 'elkjs/lib/elk-worker.min.js?worker'

export interface LayoutNode {
  id: string
  width: number
  height: number
}

/**
 * Edges as the *layout* should see them, which is not always how they are
 * drawn: inheritance is fed to ELK reversed so supertypes land above their
 * subtypes, matching UML convention.
 */
export interface LayoutEdge {
  id: string
  source: string
  target: string
}

export interface LayoutPosition {
  id: string
  x: number
  y: number
}

/**
 * ELK runs in a worker of its own, so laying out a large graph does not block
 * the main thread even though this module lives on it. The wasm parser has its
 * own separate worker; the two never interact.
 */
const elk = new ELK({
  workerFactory: () => new ElkEngineWorker() as unknown as Worker,
})

const LAYOUT_OPTIONS = {
  'elk.algorithm': 'layered',
  'elk.direction': 'DOWN',
  'elk.edgeRouting': 'POLYLINE',
  'elk.layered.spacing.nodeNodeBetweenLayers': '90',
  'elk.spacing.nodeNode': '56',
  'elk.spacing.edgeNode': '32',
  'elk.layered.nodePlacement.strategy': 'BRANDES_KOEPF',
  // Isolated types are common (utility classes, enums, annotations); packing
  // the components keeps them from stretching the canvas into a long strip.
  'elk.separateConnectedComponents': 'true',
  'elk.spacing.componentComponent': '80',
}

export async function layoutGraph(
  nodes: LayoutNode[],
  edges: LayoutEdge[],
): Promise<LayoutPosition[]> {
  const result = await elk.layout({
    id: 'root',
    layoutOptions: LAYOUT_OPTIONS,
    children: nodes.map((node) => ({
      id: node.id,
      width: node.width,
      height: node.height,
    })),
    edges: edges.map((edge) => ({
      id: edge.id,
      sources: [edge.source],
      targets: [edge.target],
    })),
  })

  return (result.children ?? []).map((child) => ({
    id: child.id,
    x: child.x ?? 0,
    y: child.y ?? 0,
  }))
}
