# VisualizeJava — frontend

Drop a Java project folder in the browser and get its type dependency graph.
Everything runs client-side: the Rust parser is compiled to WebAssembly and no
file ever leaves the tab.

## Running it

```sh
bun install
bun run dev
```

The wasm package is a build artifact and is **not** in git (`wasm/pkg` and
`src/pkg` are both gitignored by wasm-pack). If `src/pkg` is missing or stale:

```sh
bun run sync-wasm   # rebuilds ../wasm with wasm-pack and copies pkg -> src/pkg
```

Other scripts: `bun run build` (typecheck + production bundle),
`bun run preview`, `bun run lint`.

## Stack

React 19, React Flow (`@xyflow/react`), ELK for layout, Tailwind v4 with
shadcn/ui components, Vite. Dark mode only — there is no light palette and no
theme toggle.

## How it fits together

```
folder drop / picker
   └─ lib/collect-files.ts     walks the directory, keeps *.java, rejects non-folders
        └─ wasm/client.ts      promise wrapper over the parsing worker
             └─ wasm/worker.ts ProjectBuilder.add_file per file, then build_graph
                  └─ lib/graph-model.ts   wasm Map -> ProjectGraph (flat, deduped)
                       └─ lib/flow-model.ts  ProjectGraph + view state -> React Flow
                            └─ lib/layout.ts  ELK positions
```

Two workers are in play, and the split is not arbitrary:

- **`wasm/worker.ts`** parses. A multi-thousand-file project would otherwise
  freeze the tab for seconds.
- **ELK gets its own worker**, spawned from `lib/layout.ts` on the main thread.
  ELK's engine (`elk-worker.min.js`) checks whether it is running in a worker
  and, if so, installs itself as *that worker's* `onmessage` handler instead of
  exporting anything — so it cannot be imported into the parsing worker. It has
  to be a worker entry (`?worker`) in its own right.

### Design decisions worth knowing

**A bad file aborts the whole import.** `add_file` throws on a parse failure and
nothing is graphed; the dialog names the file and the error. The parser targets
Java 8, so anything newer will stop the import rather than silently produce a
partial graph.

**Associations are hidden by default.** They outnumber inheritance edges roughly
10:1 (on a 209-file sample: 96 inheritance vs 512 association relationships
after deduplication), and showing them by default turns the graph into a
hairball. Parallel associations between the same pair collapse into one edge
that remembers every member that produced it.

**Relationships are encoded by line style, not colour** — solid + closed arrow
for `extends`, dashed + closed for `implements`, dotted + open for association.
Colour is reserved for packages, and the toolbar toggles double as the legend.

**Only three packages get a colour.** A node-link diagram puts arbitrary pairs
of nodes side by side, so the palette must clear the all-pairs colour-separation
gate. Against this dark surface exactly three hues do (blue / orange / aqua);
any fourth fails. The three largest packages take them and the rest are neutral.
Colour is never the only channel — the package name is on every node, and
clicking a legend entry searches that package, which is how you pick out a
neutral one.

**Layout runs only on structural change.** Toggling relationship kinds or
"hide unconnected" changes which nodes exist and triggers a fresh ELK pass;
search and neighbourhood isolation only restyle, so the graph never jumps under
the cursor. Inheritance edges are fed to ELK reversed so supertypes land above
their subtypes, UML-style, while the drawn arrow still points at the supertype.

**Node sizes are computed, not measured** (`lib/node-size.ts`). ELK needs
dimensions before anything renders; a measure-then-relayout round trip would
make the graph visibly jump.

## Layout of the source

| Path | What lives there |
|---|---|
| `types/wasm-graph.ts` | TypeScript mirror of the Rust serde output |
| `types/graph.ts` | the flat domain model the UI renders |
| `lib/java.ts` | rendering Java signatures for the detail panel |
| `lib/palette.ts` | package colours, relationship line styles |
| `components/` | app components; `components/ui/` is shadcn |
