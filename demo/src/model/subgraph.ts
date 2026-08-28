import {
  memberSlot,
  type AssociationModel,
  type GraphModel,
  type TypeModel,
} from '@/model/build'

export interface FocusOptions {
  /** Inheritance is sparse — 96 edges over 257 types in `prod` — so 2 is cheap. */
  inheritanceHops?: number
  /** Association is dense; 2 hops takes the p90 neighbourhood from 8 to 49. */
  associationHops?: number
}

export const DEFAULT_FOCUS: Required<FocusOptions> = {
  inheritanceHops: 2,
  associationHops: 1,
}

/**
 * The types in one package, as a graph in its own right.
 *
 * Nesting closure cannot pull in a foreign package: a nested type always carries
 * its outer type's package.
 */
export function packageSubgraph(model: GraphModel, packageName: string): GraphModel {
  const seed: string[] = []
  for (const type of model.types.values()) {
    if (type.packageName === packageName) seed.push(type.key)
  }
  return subsetOf(model, seed)
}

/**
 * A readable neighbourhood around one type.
 *
 * The two relations get different hop budgets because they have very different
 * densities — see `DEFAULT_FOCUS`. Both walks start from the focus rather than
 * compounding, so the result stays close to the measured distribution (p90 of 11
 * for 2-hop inheritance, 8 for 1-hop association).
 *
 * Edges are followed in both directions: "what does this extend" and "what
 * extends this" are equally part of understanding a type.
 */
export function focusSubgraph(
  model: GraphModel,
  focusKey: string,
  options: FocusOptions = {},
): GraphModel {
  const { inheritanceHops, associationHops } = { ...DEFAULT_FOCUS, ...options }

  const seed = new Set<string>([focusKey])
  for (const key of reach(inheritanceAdjacency(model), focusKey, inheritanceHops)) {
    seed.add(key)
  }
  for (const key of reach(associationAdjacency(model), focusKey, associationHops)) {
    seed.add(key)
  }

  return subsetOf(model, seed)
}

/**
 * How many neighbours each visible type has that the view left out.
 *
 * This is what the `+N` badge reports: without it a focus view silently implies
 * the boundary types have no other relationships.
 */
export function hiddenNeighbourCounts(
  model: GraphModel,
  view: GraphModel,
): Map<string, number> {
  const inheritance = inheritanceAdjacency(model)
  const association = associationAdjacency(model)
  const counts = new Map<string, number>()

  for (const key of view.types.keys()) {
    let hidden = 0
    for (const neighbour of inheritance.get(key) ?? []) {
      if (!view.types.has(neighbour)) hidden += 1
    }
    for (const neighbour of association.get(key) ?? []) {
      if (!view.types.has(neighbour) && !(inheritance.get(key)?.has(neighbour) ?? false)) {
        hidden += 1
      }
    }
    if (hidden > 0) counts.set(key, hidden)
  }

  return counts
}

// --- internals ---------------------------------------------------------------

type Adjacency = Map<string, Set<string>>

function inheritanceAdjacency(model: GraphModel): Adjacency {
  const adjacency: Adjacency = new Map()
  for (const edge of model.inheritance) {
    link(adjacency, edge.source, edge.target)
    link(adjacency, edge.target, edge.source)
  }
  return adjacency
}

function associationAdjacency(model: GraphModel): Adjacency {
  const adjacency: Adjacency = new Map()
  for (const assoc of model.associations) {
    if (assoc.ownerKey === assoc.targetKey) continue
    link(adjacency, assoc.ownerKey, assoc.targetKey)
    link(adjacency, assoc.targetKey, assoc.ownerKey)
  }
  return adjacency
}

function link(adjacency: Adjacency, from: string, to: string) {
  const bucket = adjacency.get(from)
  if (bucket) bucket.add(to)
  else adjacency.set(from, new Set([to]))
}

/** Breadth-first, excluding the start. */
function reach(adjacency: Adjacency, start: string, hops: number): Set<string> {
  const seen = new Set<string>([start])
  let frontier = new Set<string>([start])

  for (let hop = 0; hop < hops; hop += 1) {
    const next = new Set<string>()
    for (const key of frontier) {
      for (const neighbour of adjacency.get(key) ?? []) {
        if (!seen.has(neighbour)) {
          seen.add(neighbour)
          next.add(neighbour)
        }
      }
    }
    if (next.size === 0) break
    frontier = next
  }

  seen.delete(start)
  return seen
}

/**
 * Builds a `GraphModel` over a subset of types.
 *
 * Returning the same shape is the whole point: `layoutGraph`, `TypeNode`,
 * `geometry` and `deriveHighlight` all take a `GraphModel`, so a view is a model
 * transform rather than a second rendering path.
 *
 * `TypeModel`s are shared by reference, not copied — including `isolated`, which
 * stays a property of the *whole* graph. A type that carries no inheritance
 * anywhere should read as compact in every view; one that merely has no visible
 * edge here should not.
 */
function subsetOf(model: GraphModel, seed: Iterable<string>): GraphModel {
  const keys = closeOverNesting(model, seed)

  const types = new Map<string, TypeModel>()
  for (const key of keys) {
    const type = model.types.get(key)
    if (type) types.set(key, type)
  }

  const rootKeys = model.rootKeys.filter((key) => types.has(key))
  const inheritance = model.inheritance.filter(
    (edge) => types.has(edge.source) && types.has(edge.target),
  )
  const associations = model.associations.filter(
    (assoc) => types.has(assoc.ownerKey) && types.has(assoc.targetKey),
  )

  const byMember = new Map<string, AssociationModel[]>()
  const byTarget = new Map<string, AssociationModel[]>()
  const byOwner = new Map<string, AssociationModel[]>()
  for (const assoc of associations) {
    push(byMember, memberSlot(assoc.ownerKey, assoc.memberKey), assoc)
    push(byTarget, assoc.targetKey, assoc)
    push(byOwner, assoc.ownerKey, assoc)
  }

  return { types, rootKeys, inheritance, associations, byMember, byTarget, byOwner }
}

/**
 * Expands each seed to its whole nesting family — outermost ancestor plus every
 * descendant.
 *
 * `layoutGraph` recurses `type.childKeys` and dereferences each one, so a view
 * holding `Book` but not `Book.Builder` would crash. `prod` has 48 nested types
 * over 209 roots, so pulling in whole families costs almost nothing.
 */
function closeOverNesting(model: GraphModel, seed: Iterable<string>): Set<string> {
  const included = new Set<string>()

  for (const key of seed) {
    const stack = [outermost(model, key)]
    while (stack.length > 0) {
      const current = stack.pop()!
      if (included.has(current)) continue
      included.add(current)
      const type = model.types.get(current)
      if (type) stack.push(...type.childKeys)
    }
  }

  return included
}

function outermost(model: GraphModel, key: string): string {
  let cursor = key
  for (let guard = 0; guard < 16; guard += 1) {
    const parent = model.types.get(cursor)?.parentKey
    if (!parent) return cursor
    cursor = parent
  }
  return cursor
}

function push<T>(map: Map<string, T[]>, key: string, value: T) {
  const bucket = map.get(key)
  if (bucket) bucket.push(value)
  else map.set(key, [value])
}
