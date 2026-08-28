/**
 * Scale probe: measures a real payload through the real layout, at every level.
 *
 * Not part of the app bundle and not part of `npm run verify` — it needs a graph
 * that is far too large to commit as a fixture. Capture one from the running
 * backend and point at it:
 *
 *   GRAPH=/tmp/prod.json npm run scale
 *
 * The number that matters is the L3 distribution: it is what decides whether the
 * focus hop budgets are right.
 */
import { readFileSync } from 'node:fs'

import type { RawGraph } from '@/api/types'
import { layoutGraph, layoutPackages } from '@/flow/layout'
import { buildGraphModel } from '@/model/build'
import { buildPackageGraph } from '@/model/packages'
import { focusSubgraph, packageSubgraph } from '@/model/subgraph'
import { deriveHighlight } from '@/state/selection'

const path = process.env.GRAPH
if (!path) {
  console.error('set GRAPH=<path to a captured POST /graph response>')
  process.exit(2)
}

const model = buildGraphModel(JSON.parse(readFileSync(path, 'utf8')) as RawGraph)

const VIEWPORTS: [number, number][] = [
  [1600, 900],
  [2560, 1400],
]

const isolated = [...model.types.values()].filter((t) => t.isolated).length
console.log(
  `graph: ${model.types.size} types (${isolated} isolated), ` +
    `${model.inheritance.length} inheritance, ${model.associations.length} associations`,
)

// --- L1: package overview ----------------------------------------------------

const packages = buildPackageGraph(model)
const density =
  (100 * packages.edges.length) /
  Math.max(1, packages.packages.size * (packages.packages.size - 1))
const inheritanceEdges = packages.edges.filter((e) => e.inheritance > 0).length
console.log(
  `\nL1 packages: ${packages.packages.size} nodes, ${packages.edges.length} edges ` +
    `(${density.toFixed(1)}% density, ${inheritanceEdges} carry inheritance)`,
)
report('   ', await layoutPackages(packages))

// --- L2: the largest package -------------------------------------------------

const biggest = [...packages.packages.values()].sort((a, b) => b.total - a.total)[0]
const sizes = [...packages.packages.values()].map((p) => p.total).sort((a, b) => a - b)
console.log(
  `\nL2 package contents: p50 ${sizes[sizes.length >> 1]}, max ${sizes[sizes.length - 1]} ` +
    `(${biggest.name})`,
)
const pkgFits: { name: string; zoom: number }[] = []
for (const pkg of packages.packages.values()) {
  pkgFits.push({
    name: pkg.name,
    zoom: fitZoom(await layoutGraph(packageSubgraph(model, pkg.name))),
  })
}
pkgFits.sort((a, b) => a.zoom - b.zoom)
console.log(
  `   fit zoom @1600x900: worst ${pkgFits[0].zoom.toFixed(2)} (${pkgFits[0].name}), ` +
    `below 0.5: ${pkgFits.filter((p) => p.zoom < 0.5).length}/${pkgFits.length}`,
)
report('   biggest: ', await layoutGraph(packageSubgraph(model, biggest.name)))

// --- L3: focus neighbourhoods, across every possible focus -------------------

const focusSizes: { key: string; size: number }[] = []
for (const key of model.types.keys()) {
  focusSizes.push({ key, size: focusSubgraph(model, key).types.size })
}
focusSizes.sort((a, b) => a.size - b.size)
const at = (q: number) => focusSizes[Math.min(focusSizes.length - 1, Math.floor(focusSizes.length * q))]
const worst = focusSizes[focusSizes.length - 1]
console.log(
  `\nL3 focus views: p50 ${at(0.5).size}, p90 ${at(0.9).size}, ` +
    `max ${worst.size} (${worst.key})`,
)

// Node count is not the whole story: a 40-node view laid out as one wide row is
// as unreadable as the whole graph. Fit zoom is what actually decides.
const fits: { key: string; zoom: number; size: number }[] = []
for (const { key, size } of focusSizes) {
  fits.push({ key, size, zoom: fitZoom(await layoutGraph(focusSubgraph(model, key))) })
}
fits.sort((a, b) => a.zoom - b.zoom)
const zoomAt = (q: number) => fits[Math.min(fits.length - 1, Math.floor(fits.length * q))]
console.log(
  `   fit zoom @1600x900: p10 ${zoomAt(0.1).zoom.toFixed(2)}, ` +
    `p50 ${zoomAt(0.5).zoom.toFixed(2)}, worst ${fits[0].zoom.toFixed(2)} (${fits[0].key})`,
)
console.log(
  `   views below 0.5 zoom: ${fits.filter((f) => f.zoom < 0.5).length}/${fits.length}`,
)
report('   worst: ', await layoutGraph(focusSubgraph(model, fits[0].key)))

// --- L4: the retained whole-graph escape hatch -------------------------------

console.log('\nL4 every type (escape hatch):')
report('   ', await layoutGraph(model))

// --- selection blast radius --------------------------------------------------

let worstEdges = 0
let worstEdgeKey = ''
for (const key of model.types.keys()) {
  const n = deriveHighlight({ kind: 'type', typeKey: key }, model).links.length
  if (n > worstEdges) {
    worstEdges = n
    worstEdgeKey = key
  }
}
const worstFanIn = Math.max(
  ...[...model.types.keys()].map((k) => (model.byTarget.get(k) ?? []).length),
)
console.log(
  `\nworst type-click: ${worstEdges} aggregated edges (${worstEdgeKey}); ` +
    `worst raw member fan-in ${worstFanIn}`,
)

type Placed = Map<string, { x: number; y: number; width: number; height: number; parentKey: string | null }>

function extent(layout: Placed) {
  const roots = [...layout.values()].filter((n) => !n.parentKey)
  return {
    width: Math.max(...roots.map((n) => n.x + n.width)),
    height: Math.max(...roots.map((n) => n.y + n.height)),
  }
}

function fitZoom(layout: Placed, viewport: [number, number] = VIEWPORTS[0]): number {
  const { width, height } = extent(layout)
  return Math.min(1, Math.min(viewport[0] / width, viewport[1] / height))
}

function report(prefix: string, layout: Placed) {
  const { width, height } = extent(layout)
  const zooms = VIEWPORTS.map(
    ([w, h]) => `${w}x${h}→${Math.min(1, Math.min(w / width, h / height)).toFixed(2)}`,
  ).join('  ')
  console.log(
    `${prefix}canvas ${Math.round(width)}x${Math.round(height)} ` +
      `(aspect ${(width / height).toFixed(2)})  fit: ${zooms}`,
  )
}
