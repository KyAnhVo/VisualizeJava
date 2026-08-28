import { FlaskConical } from 'lucide-react'
import { useCallback, useMemo, useState } from 'react'

import {
  BACKEND_URL,
  fetchGraph,
  GraphRequestError,
  type UploadFile,
} from '@/api/client'
import type { RawGraph } from '@/api/types'
import { SourcePicker } from '@/components/SourcePicker'
import { Button } from '@/components/ui/button'
import { GraphCanvasProvider } from '@/flow/GraphCanvas'
import { buildGraphModel } from '@/model/build'

export default function App() {
  const [raw, setRaw] = useState<RawGraph | null>(null)
  const [fileCount, setFileCount] = useState(0)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const model = useMemo(() => (raw ? buildGraphModel(raw) : null), [raw])

  const load = useCallback(async (files: UploadFile[]) => {
    setBusy(true)
    setError(null)
    try {
      const graph = await fetchGraph(files)
      setRaw(graph)
      setFileCount(files.length)
    } catch (cause) {
      setRaw(null)
      setError(describe(cause))
    } finally {
      setBusy(false)
    }
  }, [])

  const loadFixture = useCallback(async () => {
    // Dev-only shortcut. The dynamic import keeps the fixture out of the
    // production bundle entirely — the shipped demo is upload-only.
    const fixture = await import('@/model/__fixture__/small.json')
    setRaw(fixture.default as unknown as RawGraph)
    setFileCount(16)
    setError(null)
  }, [])

  const stats = useMemo(() => {
    if (!model) return null
    return {
      types: model.types.size,
      extends: model.inheritance.filter((e) => e.kind === 'Extends').length,
      implements: model.inheritance.filter((e) => e.kind === 'Implements').length,
      associations: model.associations.length,
    }
  }, [model])

  return (
    <div className="flex h-full flex-col">
      <header className="flex h-12 shrink-0 items-center gap-4 border-b border-border px-4">
        <span className="font-mono text-sm font-semibold tracking-tight">
          VisualizeJava
        </span>

        {stats && (
          <div className="flex items-center gap-3 font-mono text-[11px] text-muted-foreground">
            <Stat label="files" value={fileCount} />
            <Stat label="types" value={stats.types} />
            <Stat
              label="extends"
              value={stats.extends}
              color="var(--edge-extends)"
            />
            <Stat
              label="implements"
              value={stats.implements}
              color="var(--edge-implements)"
            />
            <Stat
              label="associations"
              value={stats.associations}
              color="var(--edge-assoc)"
            />
          </div>
        )}

        <div className="ml-auto flex items-center gap-3">
          {error && model && (
            <span className="max-w-md truncate font-mono text-[11px] text-destructive">
              {error}
            </span>
          )}
          {import.meta.env.DEV && (
            <Button size="sm" variant="ghost" onClick={loadFixture}>
              <FlaskConical />
              sample
            </Button>
          )}
          <span className="hidden font-mono text-[11px] text-muted-foreground sm:inline">
            {BACKEND_URL}
          </span>
          {model && (
            <SourcePicker compact onFiles={load} busy={busy} error={null} />
          )}
        </div>
      </header>

      <main className="min-h-0 flex-1">
        {model ? (
          <GraphCanvasProvider model={model} />
        ) : (
          <SourcePicker onFiles={load} busy={busy} error={error} />
        )}
      </main>
    </div>
  )
}

function Stat({
  label,
  value,
  color,
}: {
  label: string
  value: number
  color?: string
}) {
  return (
    <span className="flex items-center gap-1">
      {color && (
        <span
          className="h-2 w-2 rounded-[2px]"
          style={{ background: color }}
          aria-hidden
        />
      )}
      <span className="text-foreground">{value}</span>
      <span>{label}</span>
    </span>
  )
}

function describe(cause: unknown): string {
  if (cause instanceof GraphRequestError) {
    return cause.detail || `backend responded ${cause.status}`
  }
  if (cause instanceof TypeError) {
    return `could not reach the backend at ${BACKEND_URL} — is it running?`
  }
  return cause instanceof Error ? cause.message : String(cause)
}
