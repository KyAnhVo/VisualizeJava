import { useMemo } from 'react'
import {
  NEUTRAL_PACKAGE_COLOR,
  coloredPackages,
  type PackageColors,
} from '@/lib/palette'
import type { ProjectType } from '@/types/graph'

interface PackageLegendProps {
  types: ProjectType[]
  colors: PackageColors
  onSelectPackage: (packageName: string) => void
}

/**
 * Only three packages can carry a distinct hue on a dark surface without
 * becoming indistinguishable, so the rest share a neutral. The legend says so
 * plainly, and every entry is a shortcut to searching that package — which is
 * how you pick out a neutral one.
 */
export function PackageLegend({
  types,
  colors,
  onSelectPackage,
}: PackageLegendProps) {
  const { highlighted, otherCount } = useMemo(() => {
    const named = coloredPackages(colors)
    const namedSet = new Set(named)
    const counts = new Map<string, number>()
    let other = 0
    for (const type of types) {
      if (namedSet.has(type.packageName)) {
        counts.set(type.packageName, (counts.get(type.packageName) ?? 0) + 1)
      } else {
        other += 1
      }
    }
    return {
      highlighted: named.map((name) => ({
        name,
        count: counts.get(name) ?? 0,
        color: colors.get(name)!,
      })),
      otherCount: other,
    }
  }, [colors, types])

  if (highlighted.length === 0) return null

  return (
    <div className="pointer-events-auto rounded-md border border-border bg-card/90 p-2.5 text-xs backdrop-blur">
      <div className="mb-1.5 text-[10px] font-medium tracking-wide text-muted-foreground uppercase">
        Packages
      </div>
      <ul className="space-y-0.5">
        {highlighted.map((entry) => (
          <li key={entry.name}>
            <button
              type="button"
              onClick={() => onSelectPackage(entry.name)}
              className="flex w-full items-center gap-2 rounded px-1 py-0.5 text-left hover:bg-accent"
              title={`Search ${entry.name}`}
            >
              <span
                aria-hidden
                className="size-2.5 shrink-0 rounded-[2px]"
                style={{ backgroundColor: entry.color }}
              />
              <span className="min-w-0 truncate">{entry.name || '(default)'}</span>
              <span className="ml-auto shrink-0 tabular-nums text-muted-foreground">
                {entry.count}
              </span>
            </button>
          </li>
        ))}
        {otherCount > 0 && (
          <li className="flex items-center gap-2 px-1 py-0.5 text-muted-foreground">
            <span
              aria-hidden
              className="size-2.5 shrink-0 rounded-[2px]"
              style={{ backgroundColor: NEUTRAL_PACKAGE_COLOR }}
            />
            <span className="min-w-0 truncate">other packages</span>
            <span className="ml-auto shrink-0 tabular-nums">{otherCount}</span>
          </li>
        )}
      </ul>
    </div>
  )
}
