import { useCallback, useRef, useState } from 'react'
import { FolderOpen, FolderTree } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Progress } from '@/components/ui/progress'
import {
  NotADirectoryError,
  collectFromDrop,
  collectFromInput,
} from '@/lib/collect-files'
import { cn } from '@/lib/utils'
import type { BuildProgress } from '@/wasm/client'
import type { UploadedFile } from '@/wasm/protocol'

interface DropZoneProps {
  onFiles: (files: UploadedFile[]) => void
  onError: (message: string) => void
  busy: boolean
  progress: BuildProgress | null
}

const PHASE_LABEL: Record<BuildProgress['phase'], string> = {
  init: 'Starting the parser…',
  parsing: 'Parsing sources',
  building: 'Resolving names and building the graph…',
}

export function DropZone({ onFiles, onError, busy, progress }: DropZoneProps) {
  const inputRef = useRef<HTMLInputElement>(null)
  // dragenter/dragleave fire for every descendant, so track depth rather than
  // a boolean or the highlight flickers as the pointer crosses children.
  const dragDepth = useRef(0)
  const [dragging, setDragging] = useState(false)

  const handleCollected = useCallback(
    (files: UploadedFile[]) => {
      if (files.length === 0) {
        onError('That folder contains no .java files.')
        return
      }
      onFiles(files)
    },
    [onError, onFiles],
  )

  const handleDrop = useCallback(
    async (event: React.DragEvent) => {
      event.preventDefault()
      dragDepth.current = 0
      setDragging(false)
      if (busy) return
      try {
        handleCollected(await collectFromDrop(event.dataTransfer))
      } catch (error) {
        onError(
          error instanceof NotADirectoryError
            ? error.message
            : `Could not read the dropped folder: ${String(error)}`,
        )
      }
    },
    [busy, handleCollected, onError],
  )

  return (
    <div className="flex h-full w-full items-center justify-center p-8">
      <div
        onDragEnter={(event) => {
          event.preventDefault()
          dragDepth.current += 1
          setDragging(true)
        }}
        onDragOver={(event) => event.preventDefault()}
        onDragLeave={(event) => {
          event.preventDefault()
          dragDepth.current -= 1
          if (dragDepth.current <= 0) setDragging(false)
        }}
        onDrop={handleDrop}
        className={cn(
          'flex w-full max-w-xl flex-col items-center gap-6 rounded-xl border border-dashed p-12 text-center transition-colors',
          dragging ? 'border-foreground/60 bg-accent/40' : 'border-border',
        )}
      >
        <div className="rounded-full border border-border bg-card p-4">
          {dragging ? (
            <FolderOpen className="size-7 text-foreground" aria-hidden />
          ) : (
            <FolderTree className="size-7 text-muted-foreground" aria-hidden />
          )}
        </div>

        <div className="space-y-2">
          <h1 className="text-xl font-semibold tracking-tight">
            Visualize a Java project
          </h1>
          <p className="text-sm text-muted-foreground">
            Drop a project folder here to build its type dependency graph.
            Everything runs in this tab — no files leave your machine.
          </p>
        </div>

        {busy ? (
          <div className="w-full max-w-xs space-y-2">
            <Progress
              value={
                progress && progress.total > 0
                  ? (progress.done / progress.total) * 100
                  : undefined
              }
            />
            <p className="text-xs text-muted-foreground">
              {progress ? PHASE_LABEL[progress.phase] : 'Working…'}
              {progress?.phase === 'parsing' &&
                ` — ${progress.done} / ${progress.total}`}
            </p>
          </div>
        ) : (
          <>
            <Button onClick={() => inputRef.current?.click()}>
              Choose folder
            </Button>
            <p className="text-xs text-muted-foreground/70">
              Java 8 syntax. Only <code>.java</code> files are read.
            </p>
          </>
        )}

        <input
          ref={(element) => {
            inputRef.current = element
            // Not a React-known attribute; setting the property directly is
            // what makes the picker offer directories instead of files.
            if (element) element.webkitdirectory = true
          }}
          type="file"
          multiple
          className="hidden"
          onChange={(event) => {
            const { files } = event.target
            if (files) handleCollected(collectFromInput(files))
            // Allow re-picking the same folder after a failed import.
            event.target.value = ''
          }}
        />
      </div>
    </div>
  )
}
