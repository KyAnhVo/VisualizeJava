import { ChevronLeft, ChevronRight, Layers, Package, Target } from 'lucide-react'

import { Button } from '@/components/ui/button'
import type { GraphModel } from '@/model/build'
import type { Level } from '@/state/level'

/**
 * Where you are, and the way back.
 *
 * The three levels are the whole navigation model — a package overview you can
 * read at zoom 1, a package's types, and one type's neighbourhood — so the trail
 * has to be visible at all times or drilling in feels like getting lost.
 */
export function Breadcrumb({
  level,
  model,
  depth,
  onHome,
  onBack,
}: {
  level: Level
  model: GraphModel
  depth: number
  onHome: () => void
  onBack: () => void
}) {
  const crumbs = describe(level, model)

  return (
    <div className="flex items-center gap-1 rounded-lg border border-border bg-card/90 px-1.5 py-1 font-mono text-[11px] backdrop-blur">
      <Button
        size="icon-xs"
        variant="ghost"
        onClick={onBack}
        disabled={depth === 0}
        aria-label="Back"
      >
        <ChevronLeft />
      </Button>

      <button
        type="button"
        onClick={onHome}
        className="flex items-center gap-1 rounded px-1 py-0.5 text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
      >
        <Package className="size-3" />
        packages
      </button>

      {crumbs.map((crumb, index) => (
        <span key={crumb.label} className="flex items-center gap-1">
          <ChevronRight className="size-3 shrink-0 text-muted-foreground/50" />
          <span
            className={
              index === crumbs.length - 1
                ? 'flex items-center gap-1 px-1 font-semibold text-foreground'
                : 'flex items-center gap-1 px-1 text-muted-foreground'
            }
          >
            {crumb.icon}
            {crumb.label}
          </span>
        </span>
      ))}
    </div>
  )
}

function describe(level: Level, model: GraphModel) {
  switch (level.kind) {
    case 'packages':
      return []
    case 'package':
      return [
        {
          icon: <Package className="size-3" />,
          label: level.name || '(default)',
        },
      ]
    case 'focus': {
      const type = model.types.get(level.typeKey)
      const crumbs = []
      if (type?.packageName) {
        crumbs.push({
          icon: <Package className="size-3" />,
          label: type.packageName,
        })
      }
      crumbs.push({
        icon: <Target className="size-3" />,
        label: type?.displayName ?? level.typeKey,
      })
      return crumbs
    }
    case 'all':
      return [{ icon: <Layers className="size-3" />, label: 'every type' }]
  }
}
