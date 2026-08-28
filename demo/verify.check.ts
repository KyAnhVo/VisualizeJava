/**
 * Headless verification of the model + layout against a real backend payload.
 * Not part of the app bundle; run with `npm run verify`.
 */
import { readFileSync } from 'node:fs'

import { isAssociation, type RawGraph } from '@/api/types'
import { COMPACT_H, HEADER_H, measureType, SUMMARY_H } from '@/flow/geometry'
import { layoutGraph, layoutPackages, type LaidOutNode } from '@/flow/layout'
import { buildGraphModel, memberSlot, type GraphModel } from '@/model/build'
import { buildPackageGraph } from '@/model/packages'
import {
  focusSubgraph,
  hiddenNeighbourCounts,
  packageSubgraph,
} from '@/model/subgraph'
import { deriveHighlight } from '@/state/selection'

// Resolved from the working directory so the compiled bundle can live anywhere.
const raw = JSON.parse(
  readFileSync('src/model/__fixture__/small.json', 'utf8'),
) as RawGraph

let failures = 0
function check(label: string, ok: boolean, detail = '') {
  if (!ok) failures += 1
  console.log(`${ok ? '  ok  ' : ' FAIL '} ${label}${detail ? ` — ${detail}` : ''}`)
}

const model = buildGraphModel(raw)

console.log('\n# model')
check('every raw key became a type', model.types.size === Object.keys(raw).length,
  `${model.types.size}/${Object.keys(raw).length}`)

const book = model.types.get('library.model.Book')!
const builder = model.types.get('library.model.Book.Builder')!
check('Book.Builder is nested inside Book', builder.parentKey === 'library.model.Book')
check('Book lists Builder as a child', book.childKeys.includes('library.model.Book.Builder'))
check('Builder display name keeps the outer type', builder.displayName === 'Book.Builder',
  builder.displayName)
check('Builder is not a root', !model.rootKeys.includes(builder.key))
check('root count excludes nested types', model.rootKeys.length === model.types.size - 1,
  `${model.rootKeys.length} roots`)

const loanStatus = model.types.get('library.model.LoanStatus')!
check('LoanStatus is an enum with all values',
  loanStatus.kind === 'enum' &&
    loanStatus.enumValues.join(',') === 'ACTIVE,RETURNED,OVERDUE',
  loanStatus.enumValues.join(','))

console.log('\n# isolation (drives the compact node)')
check('Book takes part in inheritance, so it is not isolated', !book.isolated)
check('a nested type is never isolated', !builder.isolated)
const constants = model.types.get('library.core.Constants')!
check('Constants has no inheritance and no nesting, so it is isolated',
  constants.isolated)
const isolatedCount = [...model.types.values()].filter((t) => t.isolated).length
console.log(`  (${isolatedCount}/${model.types.size} types isolated)`)

console.log('\n# edges')
const rawOut = Object.values(raw).reduce((n, node) => n + node.out_edges.length, 0)
console.log(`  (raw out_edges=${rawOut})`)
console.log(`  (model inheritance=${model.inheritance.length}, associations=${model.associations.length})`)

const ids = new Set(model.inheritance.map((e) => e.id).concat(model.associations.map((e) => e.id)))
check('edge ids are unique', ids.size === model.inheritance.length + model.associations.length)

const unknownEndpoint = [...model.inheritance, ...model.associations].find((e) =>
  'source' in e
    ? !model.types.has(e.source) || !model.types.has(e.target)
    : !model.types.has(e.ownerKey) || !model.types.has(e.targetKey),
)
check('no edge points outside the project', !unknownEndpoint)
check('no edge to Comparable (external supertype)',
  !model.inheritance.some((e) => e.target.includes('Comparable')))

check('BookRepository extends AbstractRepository',
  model.inheritance.some((e) =>
    e.kind === 'Extends' &&
    e.source === 'library.repository.BookRepository' &&
    e.target === 'library.repository.AbstractRepository'))
check('Book implements Identifiable (interface impl is `Implements`)',
  model.inheritance.some((e) =>
    e.kind === 'Implements' &&
    e.source === 'library.model.Book' &&
    e.target === 'library.core.Identifiable'))

console.log('\n# association derivation')
// Every association the backend emitted must survive the member-walk, or the
// derivation would be trading recall for precision.
const fromBackend = new Set<string>()
for (const [ownerKey, node] of Object.entries(raw)) {
  for (const edge of node.out_edges) {
    if (isAssociation(edge.variant) && model.types.has(edge.typename.typename)) {
      fromBackend.add(`${ownerKey} -> ${edge.typename.typename}`)
    }
  }
}
const derived = new Set(model.associations.map((a) => `${a.ownerKey} -> ${a.targetKey}`))
const lost = [...fromBackend].filter((pair) => !derived.has(pair))
check('the derived set loses nothing the backend found', lost.length === 0, lost.join('; '))
console.log(`  (backend pairs=${fromBackend.size}, derived pairs=${derived.size})`)

const repo = model.types.get('library.repository.BookRepository')!
const findByGenre = repo.members.find((m) => m.name === 'findByGenre')!
const fromFindByGenre =
  model.byMember.get(memberSlot(repo.key, findByGenre.key))?.map((a) => a.targetKey) ?? []
check('findByGenre(Genre): List<Book> reaches Book through the type argument',
  fromFindByGenre.includes('library.model.Book'), fromFindByGenre.join(', '))
check('…and Genre through the parameter', fromFindByGenre.includes('library.model.Genre'))
check('BookRepository as a whole now reaches Book',
  (model.byOwner.get(repo.key) ?? []).some((a) => a.targetKey === 'library.model.Book'))

const isbn = book.members.find((m) => m.name === 'isbn')!
const fromIsbn =
  model.byMember.get(memberSlot(book.key, isbn.key))?.map((a) => a.targetKey) ?? []
check('@Field on Book.isbn is an association to the annotation type',
  fromIsbn.includes('library.annotations.Field'), fromIsbn.join(', '))
check('the annotation type is no longer orphaned',
  (model.byTarget.get('library.annotations.Field') ?? []).length > 0,
  `${(model.byTarget.get('library.annotations.Field') ?? []).length} members use it`)

const service = model.types.get('library.service.LibraryService')!
const loansField = service.members.find((m) => m.name === 'loans')!
check('LibraryService.loans (List<Loan>) reaches Loan',
  (model.byMember.get(memberSlot(service.key, loansField.key)) ?? [])
    .some((a) => a.targetKey === 'library.model.Loan'))

console.log('\n# selection')
const loan = model.types.get('library.model.Loan')!
const bookField = loan.members.find((m) => m.name === 'book')!

const fromMember = deriveHighlight(
  { kind: 'member', typeKey: loan.key, memberKey: bookField.key },
  model,
)
check('clicking Loan.book yields exactly one link',
  fromMember.links.length === 1, `${fromMember.links.length}`)
check('…pointing at Book', fromMember.links[0]?.targetKey === 'library.model.Book')
check('…and lights only Loan + Book', fromMember.litTypes.size === 2,
  [...fromMember.litTypes].join(', '))
check('…and records the member behind it',
  fromMember.links[0]?.memberKeys.join() === bookField.key)

const ontoBook = deriveHighlight({ kind: 'type', typeKey: 'library.model.Book' }, model)
check('clicking Book finds the types that use it',
  ontoBook.links.length > 0, `${ontoBook.links.length} types`)
check('…including Loan',
  ontoBook.links.some((l) => l.ownerKey === loan.key))
check('…with one edge per owner type, not per member',
  new Set(ontoBook.links.map((l) => l.id)).size === ontoBook.links.length)
check('…and every member behind a link resolves to a real row',
  ontoBook.links.every((link) =>
    link.memberKeys.every((key) => model.types.get(link.ownerKey)!.memberIndex.has(key))))

// Aggregation is the whole reason members left the canvas: the raw triple count
// must be able to exceed the edge count without anything being lost.
const rawOnBook = (model.byTarget.get('library.model.Book') ?? []).length
check('aggregation never invents or drops a member',
  ontoBook.links.reduce((n, l) => n + l.memberKeys.length, 0) === rawOnBook,
  `${rawOnBook} members over ${ontoBook.links.length} edges`)

const cleared = deriveHighlight({ kind: 'none' }, model)
check('clearing hides every association', !cleared.active && cleared.links.length === 0)

console.log('\n# geometry')
check('an isolated type collapses to a single-line header',
  measureType(constants).contentHeight === COMPACT_H,
  `${measureType(constants).contentHeight}`)
check('a connected type keeps its package line and summary row',
  measureType(loan).contentHeight === HEADER_H + SUMMARY_H,
  `${measureType(loan).contentHeight}`)
check('an enum still shows its values',
  measureType(loanStatus).enumHeight > 0)
// The point of moving members into the inspector: a type's box is a function of
// its name, its enum values and whether it is isolated — never of how many
// members it has. `test_target/prod` has a 494-member type that used to expand
// into a ~10,000px node and force a relayout of the whole graph.
check('node height is independent of member count',
  [...model.types.values()].every((type) => {
    const geometry = measureType(type)
    const base = type.isolated ? COMPACT_H : HEADER_H + SUMMARY_H
    return geometry.contentHeight === base + geometry.enumHeight
  }))
check('…so a 14-member type is the same height as a 1-member one',
  measureType(service).contentHeight === measureType(constants).contentHeight,
  `${service.members.length} vs ${constants.members.length} members`)

console.log('\n# package aggregation')
const pkgGraph = buildPackageGraph(model)
console.log(`  (${pkgGraph.packages.size} packages, ${pkgGraph.edges.length} edges)`)

check('every type lands in exactly one package',
  [...pkgGraph.packages.values()].reduce((n, p) => n + p.total, 0) === model.types.size)
check('package type lists partition the graph',
  new Set(
    [...pkgGraph.packages.values()].flatMap((p) => p.typeKeys),
  ).size === model.types.size)
check('kind counts sum to the package total',
  [...pkgGraph.packages.values()].every(
    (p) => Object.values(p.counts).reduce((a, b) => a + b, 0) === p.total,
  ))
check('no package depends on itself',
  pkgGraph.edges.every((e) => e.source !== e.target))
check('Book.Builder is filed under its package, not its outer type',
  pkgGraph.packages.get('library.model')!.typeKeys.includes('library.model.Book.Builder'))

// Weight must be distinct *type* pairs. Counting raw associations would make a
// package look coupled in proportion to how often one class mentions another.
const expectedAssoc = new Map<string, Set<string>>()
for (const assoc of model.associations) {
  const from = model.types.get(assoc.ownerKey)!.packageName
  const to = model.types.get(assoc.targetKey)!.packageName
  if (from === to) continue
  const bucket = expectedAssoc.get(`${from} ${to}`) ?? new Set()
  bucket.add(`${assoc.ownerKey} ${assoc.targetKey}`)
  expectedAssoc.set(`${from} ${to}`, bucket)
}
check('association weight counts type pairs, not member triples',
  pkgGraph.edges.every(
    (e) => e.association === (expectedAssoc.get(`${e.source} ${e.target}`)?.size ?? 0),
  ))
check('every package edge carries at least one type pair',
  pkgGraph.edges.every((e) => e.weight === e.inheritance + e.association && e.weight > 0))

const pkgLayout = await layoutPackages(pkgGraph)
check('every package was laid out', pkgLayout.size === pkgGraph.packages.size,
  `${pkgLayout.size}/${pkgGraph.packages.size}`)
check('package boxes have a positive size',
  [...pkgLayout.values()].every((n) => n.width > 0 && n.height > 0))

console.log('\n# subgraphs')
const modelPkg = packageSubgraph(model, 'library.model')
check('a package subgraph holds exactly that package',
  [...modelPkg.types.values()].every((t) => t.packageName === 'library.model'),
  [...new Set([...modelPkg.types.values()].map((t) => t.packageName))].join(', '))
check('…including every type in it',
  modelPkg.types.size ===
    [...model.types.values()].filter((t) => t.packageName === 'library.model').length)
check('…and it is nesting-closed', modelPkg.types.has('library.model.Book.Builder'))

const focusLoan = focusSubgraph(model, 'library.model.Loan')
check('a focus subgraph contains its focus', focusLoan.types.has('library.model.Loan'))
checkClosed('focus(Loan)', focusLoan)
checkClosed('package(library.model)', modelPkg)

// Contract: with no hops allowed, the view is exactly the focus' nesting family.
const alone = focusSubgraph(model, 'library.model.Book', {
  inheritanceHops: 0,
  associationHops: 0,
})
check('zero hops yields just the focus and its nesting family',
  [...alone.types.keys()].sort().join(',') ===
    ['library.model.Book', 'library.model.Book.Builder'].join(','),
  [...alone.types.keys()].join(','))

// Contract: hop budgets are monotone — a wider budget can only add types.
const narrow = focusSubgraph(model, 'library.model.Book', {
  inheritanceHops: 1,
  associationHops: 0,
})
const wide = focusSubgraph(model, 'library.model.Book', {
  inheritanceHops: 2,
  associationHops: 1,
})
check('a wider hop budget is a superset of a narrower one',
  [...narrow.types.keys()].every((k) => wide.types.has(k)),
  `${narrow.types.size} ⊄ ${wide.types.size}`)

// Contract: nothing appears that is further than the budget allows. Distances
// are recomputed here from the documented semantics — undirected, both walks
// starting at the focus — rather than from the subgraph builder.
const withinBudget = [...wide.types.keys()].every((key) => {
  if (wide.types.get(key)!.parentKey) return true // pulled in by nesting closure
  return distance(model, 'library.model.Book', key, 'inheritance') <= 2 ||
    distance(model, 'library.model.Book', key, 'association') <= 1
})
check('no type exceeds its relation\'s hop budget', withinBudget)

const boundary = hiddenNeighbourCounts(model, wide)
check('the boundary reports neighbours the view left out',
  [...boundary.values()].every((n) => n > 0))
check('…and never counts a type that is on screen',
  [...boundary.keys()].every((k) => wide.types.has(k)))

const subLayout = await layoutGraph(wide)
check('a subgraph is a valid GraphModel — it lays out',
  subLayout.size === wide.types.size, `${subLayout.size}/${wide.types.size}`)

console.log('\n# layout')
await verifyLayout()

function checkClosed(label: string, view: GraphModel) {
  check(`${label}: no edge dangles outside the view`,
    view.inheritance.every((e) => view.types.has(e.source) && view.types.has(e.target)) &&
      view.associations.every(
        (a) => view.types.has(a.ownerKey) && view.types.has(a.targetKey),
      ))
  check(`${label}: nesting is closed upward and downward`,
    [...view.types.values()].every(
      (t) =>
        (!t.parentKey || view.types.has(t.parentKey)) &&
        t.childKeys.every((c) => view.types.has(c)),
    ))
  check(`${label}: roots are the parentless types`,
    view.rootKeys.length ===
      [...view.types.values()].filter((t) => !t.parentKey).length)
}

/** Undirected BFS distance over one relation; Infinity when unreachable. */
function distance(
  graph: GraphModel,
  from: string,
  to: string,
  relation: 'inheritance' | 'association',
): number {
  const adjacency = new Map<string, Set<string>>()
  const link = (a: string, b: string) => {
    if (!adjacency.has(a)) adjacency.set(a, new Set())
    adjacency.get(a)!.add(b)
  }
  if (relation === 'inheritance') {
    for (const e of graph.inheritance) {
      link(e.source, e.target)
      link(e.target, e.source)
    }
  } else {
    for (const a of graph.associations) {
      if (a.ownerKey === a.targetKey) continue
      link(a.ownerKey, a.targetKey)
      link(a.targetKey, a.ownerKey)
    }
  }

  const seen = new Set([from])
  let frontier = [from]
  let steps = 0
  while (frontier.length > 0) {
    if (seen.has(to) && steps > 0) break
    const next: string[] = []
    for (const key of frontier) {
      for (const neighbour of adjacency.get(key) ?? []) {
        if (seen.has(neighbour)) continue
        seen.add(neighbour)
        next.push(neighbour)
      }
    }
    steps += 1
    if (next.includes(to)) return steps
    frontier = next
  }
  return from === to ? 0 : Infinity
}

async function verifyLayout() {
  const layout = await layoutGraph(model)

  check('every type was laid out', layout.size === model.types.size,
    `${layout.size}/${model.types.size}`)
  check('every node has a positive size',
    [...layout.values()].every((n) => n.width > 0 && n.height > 0))

  for (const node of layout.values()) {
    const type = model.types.get(node.key)!
    const geometry = measureType(type)
    if (!type.childKeys.length) {
      check(`${type.displayName}: box matches computed content height`,
        node.height === geometry.contentHeight,
        `${node.height} vs ${geometry.contentHeight}`)
    } else {
      check(`${type.displayName}: container leaves room for its own content`,
        node.height > geometry.contentHeight,
        `${node.height} vs ${geometry.contentHeight}`)
    }
  }

  // Nested types must sit inside their parent, below the parent's own content.
  for (const node of layout.values()) {
    if (!node.parentKey) continue
    const parent = layout.get(node.parentKey)!
    const parentGeometry = measureType(model.types.get(parent.key)!)
    const label = `${model.types.get(node.key)!.displayName} inside ${model.types.get(parent.key)!.displayName}`
    check(`${label}: fits horizontally`,
      node.x >= 0 && node.x + node.width <= parent.width,
      `${node.x}+${node.width} <= ${parent.width}`)
    check(`${label}: fits vertically`,
      node.y + node.height <= parent.height,
      `${node.y}+${node.height} <= ${parent.height}`)
    check(`${label}: starts below the parent's own content`,
      node.y >= parentGeometry.contentHeight,
      `${node.y} >= ${parentGeometry.contentHeight}`)
  }

  // Siblings must not overlap, or boxes would visually collide.
  const bySiblingGroup = new Map<string, LaidOutNode[]>()
  for (const node of layout.values()) {
    const group = node.parentKey ?? '<root>'
    bySiblingGroup.set(group, [...(bySiblingGroup.get(group) ?? []), node])
  }
  let overlaps = 0
  for (const siblings of bySiblingGroup.values()) {
    for (let i = 0; i < siblings.length; i += 1) {
      for (let j = i + 1; j < siblings.length; j += 1) {
        if (intersects(siblings[i], siblings[j])) overlaps += 1
      }
    }
  }
  check('no sibling boxes overlap', overlaps === 0, `${overlaps} overlapping pairs`)

  // With direction UP, a supertype must end up above its subtype.
  const inverted = model.inheritance.filter((edge) => {
    const sub = layout.get(edge.source)!
    const sup = layout.get(edge.target)!
    return sup.absY >= sub.absY
  })
  check('supertypes are drawn above their subtypes',
    inverted.length === 0,
    inverted.map((e) => `${e.source} -> ${e.target}`).join('; '))

  const roots = [...layout.values()].filter((n) => !n.parentKey)
  const width = Math.max(...roots.map((n) => n.x + n.width))
  const height = Math.max(...roots.map((n) => n.y + n.height))
  console.log(`  (canvas ${Math.round(width)}x${Math.round(height)}, aspect ${(width / height).toFixed(2)})`)
}

function intersects(a: LaidOutNode, b: LaidOutNode): boolean {
  return (
    a.x < b.x + b.width &&
    b.x < a.x + a.width &&
    a.y < b.y + b.height &&
    b.y < a.y + a.height
  )
}

console.log(`\n${failures === 0 ? 'ALL CHECKS PASSED' : `${failures} CHECK(S) FAILED`}`)
process.exit(failures === 0 ? 0 : 1)
