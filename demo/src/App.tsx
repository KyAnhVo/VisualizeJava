import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { DetailPanel } from '@/components/DetailPanel'
import { DropZone } from '@/components/DropZone'
import { GraphCanvas, type FocusRequest } from '@/components/GraphCanvas'
import {
  ImportErrorDialog,
  type ImportError,
} from '@/components/ImportErrorDialog'
import { PackageLegend } from '@/components/PackageLegend'
import { Toolbar } from '@/components/Toolbar'
import { rootNameOf } from '@/lib/collect-files'
import {
  selectVisible,
  toFlowEdges,
  toFlowNodes,
  toLayoutInput,
  type ViewState,
} from '@/lib/flow-model'
import { layoutGraph } from '@/lib/layout'
import { assignPackageColors } from '@/lib/palette'
import type { ProjectGraph, RelationKind } from '@/types/graph'
import { BuildFailure, graphWorker, type BuildProgress } from '@/wasm/client'
import type { UploadedFile } from '@/wasm/protocol'

/** Associations outnumber inheritance ~10:1, so they start hidden. */
const DEFAULT_KINDS: RelationKind[] = ['extends', 'implements']

interface LoadedProject {
  name: string
  graph: ProjectGraph
  parsedFiles: number
}

export default function App() {
  const [project, setProject] = useState<LoadedProject | null>(null)
  const [progress, setProgress] = useState<BuildProgress | null>(null)
  const [importing, setImporting] = useState(false)
  const [error, setError] = useState<ImportError | null>(null)

  const [visibleKinds, setVisibleKinds] = useState(
    () => new Set<RelationKind>(DEFAULT_KINDS),
  )
  const [hideUnconnected, setHideUnconnected] = useState(false)
  const [search, setSearch] = useState('')
  const [selectedId, setSelectedId] = useState<string | null>(null)
  const [isolatedId, setIsolatedId] = useState<string | null>(null)

  const [positions, setPositions] = useState<Map<string, { x: number; y: number }>>(
    () => new Map(),
  )
  const [layingOut, setLayingOut] = useState(false)
  // Incremented on every completed layout so the canvas re-fits; the initial
  // fitView runs before positions exist and would otherwise leave the view
  // zoomed into the origin.
  const [fitSignal, setFitSignal] = useState(0)
  const [focus, setFocus] = useState<FocusRequest | null>(null)
  const focusNonce = useRef(0)

  const requestFocus = useCallback((ids: string[]) => {
    if (ids.length === 0) return
    setFocus({ ids, nonce: ++focusNonce.current })
  }, [])

  const startImport = useCallback(async (files: UploadedFile[]) => {
    setImporting(true)
    setProgress(null)
    try {
      const result = await graphWorker.build(files, setProgress)
      setProject({
        name: rootNameOf(files),
        graph: result.graph,
        parsedFiles: result.parsedFiles,
      })
      setSelectedId(null)
      setIsolatedId(null)
      setSearch('')
    } catch (failure) {
      setError({
        message:
          failure instanceof Error ? failure.message : String(failure),
        path: failure instanceof BuildFailure ? failure.path : null,
      })
    } finally {
      setImporting(false)
      setProgress(null)
    }
  }, [])

  const graph = project?.graph ?? null

  const packageColors = useMemo(
    () => assignPackageColors(graph?.types ?? []),
    [graph],
  )

  // Structural filters only. Isolation and search restyle without changing
  // which nodes exist, so they must not land here or every keystroke would
  // trigger a relayout and the graph would jump around.
  const visible = useMemo(
    () =>
      graph
        ? selectVisible(graph, visibleKinds, hideUnconnected)
        : { types: [], relations: [] },
    [graph, hideUnconnected, visibleKinds],
  )

  useEffect(() => {
    if (visible.types.length === 0) {
      setPositions(new Map())
      return
    }

    let cancelled = false
    setLayingOut(true)
    const { nodes, edges } = toLayoutInput(visible)
    layoutGraph(nodes, edges)
      .then((laid) => {
        if (cancelled) return
        setPositions(new Map(laid.map((p) => [p.id, { x: p.x, y: p.y }])))
        setFitSignal((n) => n + 1)
      })
      .catch((failure: unknown) => {
        if (cancelled) return
        setError({
          message: `Layout failed: ${failure instanceof Error ? failure.message : String(failure)}`,
          path: null,
        })
      })
      .finally(() => {
        if (!cancelled) setLayingOut(false)
      })

    // A newer filter change supersedes this layout; ignore its late result.
    return () => {
      cancelled = true
    }
  }, [visible])

  const searchMatches = useMemo(() => {
    const term = search.trim().toLowerCase()
    if (!term) return null
    return new Set(
      visible.types
        .filter((type) => type.id.toLowerCase().includes(term))
        .map((type) => type.id),
    )
  }, [search, visible.types])

  const view: ViewState = useMemo(
    () => ({
      visibleKinds,
      hideUnconnected,
      isolatedId,
      searchMatches,
      selectedId,
    }),
    [hideUnconnected, isolatedId, searchMatches, selectedId, visibleKinds],
  )

  const flowNodes = useMemo(
    () => toFlowNodes(visible, positions, packageColors, view),
    [packageColors, positions, view, visible],
  )
  const flowEdges = useMemo(() => toFlowEdges(visible, view), [view, visible])

  const selectedType = useMemo(
    () => graph?.types.find((type) => type.id === selectedId) ?? null,
    [graph, selectedId],
  )

  const handleSelect = useCallback(
    (id: string | null) => {
      setSelectedId(id)
      // Isolation follows the selection rather than persisting on a node the
      // user has navigated away from.
      setIsolatedId((current) => (current === null ? null : id))
      if (id) requestFocus([id])
    },
    [requestFocus],
  )

  const handleSearchSubmit = useCallback(() => {
    if (searchMatches && searchMatches.size > 0) {
      requestFocus([...searchMatches])
    }
  }, [requestFocus, searchMatches])

  // Framing the matches on every keystroke is jarring; do it once the term
  // settles.
  useEffect(() => {
    if (!searchMatches || searchMatches.size === 0) return
    const timer = setTimeout(handleSearchSubmit, 350)
    return () => clearTimeout(timer)
  }, [handleSearchSubmit, searchMatches])

  const toggleKind = useCallback((kind: RelationKind) => {
    setVisibleKinds((current) => {
      const next = new Set(current)
      if (next.has(kind)) next.delete(kind)
      else next.add(kind)
      return next
    })
  }, [])

  const reset = useCallback(() => {
    setProject(null)
    setPositions(new Map())
    setSelectedId(null)
    setIsolatedId(null)
    setSearch('')
    setVisibleKinds(new Set(DEFAULT_KINDS))
    setHideUnconnected(false)
  }, [])

  if (!project) {
    return (
      <>
        <DropZone
          onFiles={startImport}
          onError={(message) => setError({ message, path: null })}
          busy={importing}
          progress={progress}
        />
        <ImportErrorDialog error={error} onDismiss={() => setError(null)} />
      </>
    )
  }

  return (
    <div className="flex h-full flex-col">
      <Toolbar
        projectName={`${project.name} · ${project.parsedFiles} files`}
        typeCount={project.graph.types.length}
        visibleTypeCount={visible.types.length}
        relationCount={visible.relations.length}
        visibleKinds={visibleKinds}
        onToggleKind={toggleKind}
        hideUnconnected={hideUnconnected}
        onHideUnconnectedChange={setHideUnconnected}
        search={search}
        onSearchChange={setSearch}
        matchCount={searchMatches?.size ?? null}
        onReset={reset}
      />

      <div className="flex min-h-0 flex-1">
        <div className="min-w-0 flex-1">
          <GraphCanvas
            nodes={flowNodes}
            edges={flowEdges}
            focus={focus}
            fitSignal={fitSignal}
            onSelect={handleSelect}
            busy={layingOut}
            overlay={
              <PackageLegend
                types={visible.types}
                colors={packageColors}
                onSelectPackage={setSearch}
              />
            }
          />
        </div>

        {selectedType && (
          <DetailPanel
            type={selectedType}
            graph={project.graph}
            isolatedId={isolatedId}
            onSelect={handleSelect}
            onToggleIsolate={() =>
              setIsolatedId((current) =>
                current === selectedType.id ? null : selectedType.id,
              )
            }
            onClose={() => {
              setSelectedId(null)
              setIsolatedId(null)
            }}
          />
        )}
      </div>

      <ImportErrorDialog error={error} onDismiss={() => setError(null)} />
    </div>
  )
}
