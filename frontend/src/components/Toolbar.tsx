import { RotateCcw, Search, X } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Separator } from '@/components/ui/separator'
import { Switch } from '@/components/ui/switch'
import { RelationSwatch } from '@/components/RelationSwatch'
import { RELATION_STYLES } from '@/lib/palette'
import { cn } from '@/lib/utils'
import type { RelationKind } from '@/types/graph'

const KINDS: RelationKind[] = ['extends', 'implements', 'association']

interface ToolbarProps {
  projectName: string
  typeCount: number
  visibleTypeCount: number
  relationCount: number
  visibleKinds: Set<RelationKind>
  onToggleKind: (kind: RelationKind) => void
  hideUnconnected: boolean
  onHideUnconnectedChange: (value: boolean) => void
  search: string
  onSearchChange: (value: string) => void
  matchCount: number | null
  onReset: () => void
}

export function Toolbar({
  projectName,
  typeCount,
  visibleTypeCount,
  relationCount,
  visibleKinds,
  onToggleKind,
  hideUnconnected,
  onHideUnconnectedChange,
  search,
  onSearchChange,
  matchCount,
  onReset,
}: ToolbarProps) {
  return (
    <header className="flex flex-wrap items-center gap-x-4 gap-y-2 border-b border-border px-4 py-2.5">
      <div className="min-w-0">
        <div className="truncate text-sm font-semibold" title={projectName}>
          {projectName}
        </div>
        <div className="text-[11px] tabular-nums text-muted-foreground">
          {visibleTypeCount === typeCount
            ? `${typeCount} types`
            : `${visibleTypeCount} of ${typeCount} types`}
          {' · '}
          {relationCount} relationships shown
        </div>
      </div>

      <div className="relative w-56">
        <Search
          className="pointer-events-none absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2 text-muted-foreground"
          aria-hidden
        />
        <Input
          value={search}
          onChange={(event) => onSearchChange(event.target.value)}
          placeholder="Search types or packages"
          aria-label="Search types or packages"
          className="h-8 pr-7 pl-8 text-xs"
        />
        {search && (
          <button
            type="button"
            onClick={() => onSearchChange('')}
            className="absolute top-1/2 right-2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
          >
            <X className="size-3.5" aria-hidden />
            <span className="sr-only">Clear search</span>
          </button>
        )}
      </div>

      {matchCount !== null && (
        <span className="text-[11px] tabular-nums text-muted-foreground">
          {matchCount} match{matchCount === 1 ? '' : 'es'}
        </span>
      )}

      <Separator orientation="vertical" className="h-6" />

      {/* These double as the relationship legend: each button draws the edge
          style it controls. */}
      <div className="flex items-center gap-1">
        {KINDS.map((kind) => {
          const active = visibleKinds.has(kind)
          return (
            <Button
              key={kind}
              variant={active ? 'secondary' : 'ghost'}
              size="sm"
              aria-pressed={active}
              onClick={() => onToggleKind(kind)}
              className={cn('h-8 gap-1.5 px-2 text-xs', !active && 'opacity-50')}
            >
              <RelationSwatch kind={kind} />
              {RELATION_STYLES[kind].label}
            </Button>
          )
        })}
      </div>

      <Separator orientation="vertical" className="h-6" />

      <div className="flex items-center gap-2">
        <Switch
          id="hide-unconnected"
          checked={hideUnconnected}
          onCheckedChange={onHideUnconnectedChange}
        />
        <Label
          htmlFor="hide-unconnected"
          className="text-xs font-normal text-muted-foreground"
        >
          Hide unconnected
        </Label>
      </div>

      <Button
        variant="ghost"
        size="sm"
        onClick={onReset}
        className="ml-auto h-8 gap-1.5 text-xs"
      >
        <RotateCcw className="size-3.5" aria-hidden />
        New project
      </Button>
    </header>
  )
}
