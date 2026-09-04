import { AlertTriangle } from 'lucide-react'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'

export interface ImportError {
  message: string
  /** The file the failure is attributable to, when there is one. */
  path: string | null
}

export function ImportErrorDialog({
  error,
  onDismiss,
}: {
  error: ImportError | null
  onDismiss: () => void
}) {
  return (
    <Dialog open={error !== null} onOpenChange={(open) => !open && onDismiss()}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <AlertTriangle className="size-4 text-destructive" aria-hidden />
            Import failed
          </DialogTitle>
          <DialogDescription>
            {error?.path
              ? 'A file could not be parsed, so the import was stopped. Nothing was graphed.'
              : 'The project could not be imported.'}
          </DialogDescription>
        </DialogHeader>

        {error?.path && (
          <div className="rounded-md border border-border bg-card p-3">
            <div className="text-[11px] tracking-wide text-muted-foreground uppercase">
              File
            </div>
            <code className="text-xs break-all">{error.path}</code>
          </div>
        )}

        <div className="rounded-md border border-border bg-card p-3">
          <div className="text-[11px] tracking-wide text-muted-foreground uppercase">
            Error
          </div>
          <code className="text-xs break-all whitespace-pre-wrap">
            {error?.message}
          </code>
        </div>

        {error?.path && (
          <p className="text-xs text-muted-foreground">
            The parser targets Java 8. Syntax introduced later — lambdas with
            inferred types in some positions, modules, records, sealed types —
            will fail here.
          </p>
        )}

        <DialogFooter>
          <Button onClick={onDismiss}>Try another folder</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
