# DEMO

## Goal:
Create a demo for the frontend of the app, where it should demonstrate the dependency graph in java.

## Tech
React, Tailwind, Shadcn, React flow

## Style
- Depends on you, but make dark mode first class (and only dark mode).
- Make the inheritance edges very clear.


## Spec: visualize
- Only draw inheritance edge. Do not draw other edges unless one of the below conditions occur.
- For inner/outer edge, embed the inner type inside the block of the outer type
- For association member-to-type-edge:
  - Make each property/method clickable on its own
  - On member clicked, dim all other unrelated types/edges, and draw association edges that relate to this member only.
- For association type-to-member edge:
  - Make the outer box of type clickable on its own
  - on type clicked, draw association edge to members (of own type or other type) that uses it. Dim all other unrelated type/edge.
- For any click not on the member/edge, reset all dim/etc.
- Treat implement and inheritance edges as the same thing, although color code it differently and make the inheritance edge thicker and more clear than implement edges.

## Spec: data representation
- Enum: Show all enum values

---

# Implementation notes

## Running it

```sh
npm install
npm run dev            # http://localhost:5173
```

The backend must be running; the demo posts to `http://localhost:8080/graph` by
default, overridable with `VITE_API_URL`.

Other scripts: `npm run build` (tsc + vite), `npm run lint` (oxlint),
`npm run verify` (headless checks — see below).

The demo is upload-only: drop a Java source tree onto the landing page, or pick
a folder. Only `.java` files are sent, each under its project-relative path,
which is what the backend keys parts on. A parse failure fails the *whole*
request server-side, so the backend's message (which names the offending file)
is surfaced verbatim rather than swallowed.

## What the payload looks like

`GET`-less: the single route is `POST /graph`, multipart in, a flat
`{ "library.model.Book": Node }` map out. Four properties of that payload shape
the whole frontend:

1. **Every edge is project-internal.** The backend drops any edge whose other
   end is not a declared project type, so `Book implements Comparable<Book>`
   produces nothing. External types still appear as text inside member rows.
2. **`out_edges` and `in_edges` mirror each other.** Inheritance is read from
   `out_edges` alone, with a dedupe, or every relationship would be drawn twice.
3. **Nesting lives in the name, not the structure.** The map is flat;
   `library.model.Book.Builder` is a sibling entry of `library.model.Book`.
   Containment is recovered by stripping the package prefix off the typename.
4. **`Implements` is not just "class implements interface"** — the backend also
   emits it for `interface A extends B`. `Extends` means class-extends-class and
   nothing else. The legend says so.

## Associations are re-derived, not taken from `out_edges`

The backend's association edges only read the *head* of a `RefType`:
`build_associative_relationshup_from_node` in `src/abstraction_graph/graph.rs`
takes `reftype.name` and stops. So `List<Book>` produces an edge to `List` —
external, therefore dropped — and none to `Book`. Annotations and type-parameter
bounds are not walked at all.

The name resolver is not at fault: `resolve_reftype` recurses through
`type_arg_list` properly, so the information is in the payload and simply goes
unused. `collectMemberTargets` (`src/model/member.ts`) walks it — type
arguments, array and wildcard bounds, `throws`, generic bounds, and member
annotations — and keeps every in-project name it finds.

Measured against the running backend, the walk is a **strict superset**: 30 → 43
edges on `test_target/small`, 1574 → 1607 on `test_target/prod`, losing nothing
in either. `verify.check.ts` asserts the "loses nothing" half against the
fixture's own `out_edges`. Without it, `BookRepository` has no edge to `Book`
despite two methods returning `List<Book>`, and `library.annotations.Field` is
completely orphaned despite six usages.

The backend pass is still read, and unioned in, as a safety net.

## Layout

ELK (`elkjs`), `layered`, `direction: UP`, hierarchical — inner types are ELK
children of their outer type, so the layer index becomes inheritance depth and
supertypes sit above their subtypes.

Only inheritance edges take part in layout. Association edges would fight that
ordering, and they are hidden until something is selected anyway.

Node sizes are **computed, not measured** (`src/flow/geometry.ts`). ELK needs a
size before anything renders, and React Flow positions from that same layout, so
the DOM is pinned to the computed geometry: every height in `TypeNode` has a
matching constant, and all text is monospace so widths are predictable. Because
node size depends only on the model, the layout runs once per graph.

**Each connected component is laid out separately.**
`elk.separateConnectedComponents` is ignored under `hierarchyHandling:
INCLUDE_CHILDREN`, which nested types need. Handed the whole graph, ELK put all
209 top-level types of `test_target/prod` into shared layers — one of them
holding every edgeless type — and returned a 41,000 × 4,000 strip with unrelated
components interleaved (fit zoom 0.04). Laying out components one at a time and
shelf-packing the results gives 6,791 × 4,100, fit zoom 0.22.

The shelf width is chosen by trying a spread of candidates and keeping whichever
lands closest to a widescreen aspect; shelf packing wastes a variable amount of
space depending on how component heights fall, and a single area-derived guess
was badly off on small graphs.

## Navigation: three levels

Rendering every type does not scale, and no amount of layout work fixes it — see
`DemoReports.md`. The canvas shows one of three slices instead, with fit zoom on
a 1600×900 viewport measured across all of `test_target/prod`:

| level | what it shows | fit zoom |
|---|---|---|
| **L1** packages | 30 boxes, 37 edges | **0.71** (1.00 on a 2560px screen) |
| **L2** one package | p50 5 types, max 55 | below 0.5 for **1 of 30** |
| **L3** one type | p50 5 nodes, p90 14, max 42 | p50 **1.00**, below 0.5 for **1 of 257** |

The single outlier at both L2 and L3 is `java.nio.Buffer` — a hub with ~21 direct
subclasses. ELK layered cannot wrap a wide layer (`elk.layered.wrapping.*` splits
the *sequence* of layers, not one row), so that row is ~6500px wide whatever the
settings. One case in 257 did not justify a custom row-reflow pass.

**L3 uses different hop budgets per relation: 2 for inheritance, 1 for
association.** Inheritance is sparse (96 edges over 257 types); association is
dense enough that a second hop takes the p90 neighbourhood from 8 nodes to 49.
Both walks start at the focus rather than compounding, and follow edges in both
directions — "what does this extend" and "what extends this" are equally part of
reading a type. Boundary nodes carry a `+N` badge; clicking one re-centres.

At L1 the type-level rule inverts: **only 5 of the 37 package edges are
inheritance**, so drawing inheritance alone up there gives a near-empty diagram.
The package view draws both relations, with thickness by weight — and weight is
*distinct type pairs*, not raw association records, or a package would look
coupled by how often one class mentions another rather than by how many classes
are involved.

`Level` (`src/state/level.ts`) also keeps the old whole-graph view as an escape
hatch. It is what this design exists to demote, but it costs almost nothing.

### A view is a filtered `GraphModel`

`layoutGraph`, `TypeNode`, `geometry` and `deriveHighlight` all take a
`GraphModel`, so `packageSubgraph` and `focusSubgraph` (`src/model/subgraph.ts`)
return that same shape and nothing downstream changes — L2 and L3 are model
transforms, not rendering variants.

Two traps that shape the code:

- **Nesting closure is mandatory.** `layoutGraph` recurses `type.childKeys` and
  dereferences each one, so a view holding `Book` but not `Book.Builder` crashes.
  Subgraphs pull in whole nesting families.
- **The inspector keeps the *full* model.** "Used by" has to report every
  referring type, including ones the current level does not draw — that is how
  you find where to navigate next. So the canvas gets `view` and the inspector
  gets `model`; off-view navigator entries are greyed and clicking one re-focuses.
  Association edges whose far end is off-view are simply not drawn.

## Reading a large graph

`test_target/prod` is 257 types, 96 inheritance edges and 1,607 associations.
Four things keep that usable:

- **Members are not on the canvas.** They live in the inspector
  (`src/components/Inspector.tsx`), which is just a scroll container. Node height
  is a function of the name, the enum values and nothing else. Previously a type
  could be expanded inline, and `prod`'s 494-member `WebGLRenderingContext`
  became a ~10,000px node that forced a full relayout.
- **Association edges are aggregated to the type level.** Clicking
  `java.nio.IntBuffer` used to mean 212 member-anchored edges and force-opening
  every owner; as distinct owner types it is 19, worst case 27 across the whole
  project. The members behind an edge are not lost — the edge is labelled with
  the count and the inspector lists them.
- **Isolated types render compactly.** 97 of `prod`'s 209 top-level types have no
  inheritance edge at all, so in the default view they are free-floating boxes.
  They collapse to a single-line header and pack into a grid.
- **Level of detail.** At a zoom that fits 257 types, 11px text renders at under
  3px. The node sheds detail as it shrinks — full card, then the name alone, then
  a solid block in the kind's colour — so an overview reads as the inheritance
  skeleton. Names are found with the search box, not by panning.

## Selection

`Selection` is one of none / type / member, and `deriveHighlight` turns it into
the lit sets and the edges to draw. The two directions of the spec are duals of
one association triple `(ownerType, member, targetType)`: a member click asks
"what does this point at", a type click asks "which members point at me".

Clicking a node opens the inspector on that type. Clicking a member in the
inspector narrows the selection to that member and swaps the navigator column to
its references; every entry there selects the type and flies the viewport to it.
Dimming keeps pointer events live, so a click can go straight from one selection
to the next.

Members are joined back to their rows by a canonical serialisation of the
`Member` object, not by name — overloads share a name but are distinct members
with distinct edges.

## Verification

`npm run verify` builds `verify.check.ts` and runs it against
`src/model/__fixture__/small.json` — a real `POST /graph` response for
`test_target/small`, captured from the running backend. It checks the model
(nesting, enum values, edge ids, no external endpoints), the association
derivation (that it loses none of the backend's edges, and that `List<Book>`,
`@Field` and `List<Loan>` each resolve), the selection logic (both click
directions, and that aggregation neither invents nor drops a member), the
geometry invariant that node height is independent of member count, and the
layout (nested boxes fit inside their parent below its own content, no sibling
overlaps, supertypes above subtypes).

It does not cover rendering. There was no browser available in this environment,
so the DOM side — the inspector, the zoom tiers, the search box — is unverified
beyond a type-check.

The fixture is dev-only: it is loaded through a dynamic import behind a `DEV`
guard, so it never enters the production bundle.
