import { FolderOpen, Loader2, TriangleAlert } from 'lucide-react'
import { useEffect, useRef, useState, type DragEvent } from 'react'

import { fromFileList, type UploadFile } from '@/api/client'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'
import { collectDroppedFiles } from '@/lib/collect-files'

export function SourcePicker({
  onFiles,
  busy,
  error,
  compact,
}: {
  onFiles: (files: UploadFile[]) => void
  busy: boolean
  error: string | null
  compact?: boolean
}) {
  const inputRef = useRef<HTMLInputElement>(null)
  const [dragging, setDragging] = useState(false)
  const [emptyDrop, setEmptyDrop] = useState(false)

  // `webkitdirectory` is not in React's typed attribute set, so it is applied
  // directly to the element.
  useEffect(() => {
    const input = inputRef.current
    if (!input) return
    input.setAttribute('webkitdirectory', '')
    input.setAttribute('directory', '')
  }, [])

  const handleDrop = async (event: DragEvent) => {
    event.preventDefault()
    setDragging(false)
    if (busy) return
    const files = await collectDroppedFiles(event.dataTransfer.items)
    setEmptyDrop(files.length === 0)
    if (files.length) onFiles(files)
  }

  const picker = (
    <input
      ref={inputRef}
      type="file"
      multiple
      accept=".java"
      className="hidden"
      onChange={(event) => {
        const files = fromFileList(event.target.files ?? [])
        setEmptyDrop(files.length === 0 && (event.target.files?.length ?? 0) > 0)
        if (files.length) onFiles(files)
        event.target.value = ''
      }}
    />
  )

  if (compact) {
    return (
      <>
        {picker}
        <Button
          size="sm"
          variant="outline"
          disabled={busy}
          onClick={() => inputRef.current?.click()}
        >
          {busy ? <Loader2 className="animate-spin" /> : <FolderOpen />}
          load another project
        </Button>
      </>
    )
  }

  return (
    <div className="grid h-full place-items-center p-6">
      <div className="w-full max-w-2xl">
        <div className="mb-8 text-center">
          <h1 className="mb-2 text-3xl font-semibold tracking-tight">
            Java dependency graph
          </h1>
          <p className="text-sm text-muted-foreground">
            Drop a Java source tree to see its types, their inheritance, and what
            each member points at.
          </p>
        </div>

        <div
          onDragOver={(event) => {
            event.preventDefault()
            setDragging(true)
          }}
          onDragLeave={() => setDragging(false)}
          onDrop={handleDrop}
          className={cn(
            'flex flex-col items-center gap-4 rounded-xl border-2 border-dashed p-12 text-center transition-colors',
            dragging
              ? 'border-primary bg-primary/5'
              : 'border-border bg-card/40 hover:border-muted-foreground/40',
          )}
        >
          {picker}
          {busy ? (
            <>
              <Loader2 className="size-7 animate-spin text-primary" />
              <p className="text-sm text-muted-foreground">
                parsing and resolving names…
              </p>
            </>
          ) : (
            <>
              <FolderOpen className="size-7 text-muted-foreground" />
              <div>
                <p className="text-sm">
                  drag a project folder here, or
                </p>
                <Button
                  variant="outline"
                  size="sm"
                  className="mt-3"
                  onClick={() => inputRef.current?.click()}
                >
                  choose a folder
                </Button>
              </div>
              <p className="font-mono text-[11px] text-muted-foreground">
                only <span className="text-foreground">.java</span> files are sent
              </p>
            </>
          )}
        </div>

        {emptyDrop && !error && (
          <p className="mt-4 text-center text-sm text-muted-foreground">
            no <span className="font-mono">.java</span> files in there.
          </p>
        )}

        {error && (
          <div className="mt-6 flex gap-3 rounded-lg border border-destructive/40 bg-destructive/10 p-4 text-sm">
            <TriangleAlert className="mt-0.5 size-4 shrink-0 text-destructive" />
            <div className="min-w-0">
              <p className="mb-1 font-medium text-destructive">
                the backend rejected the project
              </p>
              <pre className="overflow-x-auto whitespace-pre-wrap break-words font-mono text-[12px] text-muted-foreground">
                {error}
              </pre>
            </div>
          </div>
        )}
      </div>
    </div>
  )
}
