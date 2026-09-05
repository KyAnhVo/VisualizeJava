import type { ProjectType, RelationKind, TypeVariantKind } from '@/types/graph'

/**
 * Package colouring.
 *
 * A node-link diagram places arbitrary pairs of nodes side by side, so the
 * palette has to clear the *all-pairs* colour-separation gate, not the easier
 * adjacent-pairs one. Against this dark surface only three hues do:
 *
 *   blue #3987e5 / orange #d95926 / aqua #199e70
 *     worst all-pairs CVD ΔE 9.4, normal-vision ΔE 20.9, all ≥ 3:1 contrast
 *
 * Adding any fourth hue fails. Rather than ship colours users cannot tell
 * apart, the three largest packages take the three slots and everything else
 * is neutral. Colour is never the only channel: the package name is printed on
 * every node, the legend is always visible, and clicking a legend entry
 * highlights that package — which is how you pick out the neutral ones.
 */
export const PACKAGE_COLORS = ['#3987e5', '#d95926', '#199e70'] as const

/** Packages beyond the top three. */
export const NEUTRAL_PACKAGE_COLOR = 'oklch(0.708 0 0)'

export type PackageColors = Map<string, string>

/**
 * Assigns the three slots to the packages declaring the most types.
 *
 * Computed once from the whole graph and never recomputed for a filtered view:
 * colour follows the package, so filtering must not repaint the survivors.
 */
export function assignPackageColors(types: ProjectType[]): PackageColors {
  const counts = new Map<string, number>()
  for (const type of types) {
    counts.set(type.packageName, (counts.get(type.packageName) ?? 0) + 1)
  }

  const ranked = [...counts.entries()].sort(
    // Ties break on name so the assignment is deterministic across reloads.
    (a, b) => b[1] - a[1] || a[0].localeCompare(b[0]),
  )

  const colors: PackageColors = new Map()
  for (const [packageName] of ranked) {
    colors.set(
      packageName,
      PACKAGE_COLORS[colors.size] ?? NEUTRAL_PACKAGE_COLOR,
    )
  }
  return colors
}

export function packageColor(
  colors: PackageColors,
  packageName: string,
): string {
  return colors.get(packageName) ?? NEUTRAL_PACKAGE_COLOR
}

/** Packages that got a dedicated hue, in slot order, for the legend. */
export function coloredPackages(colors: PackageColors): string[] {
  return [...colors.entries()]
    .filter(([, color]) => color !== NEUTRAL_PACKAGE_COLOR)
    .map(([name]) => name)
}

export const VARIANT_LABEL: Record<TypeVariantKind, string> = {
  class: 'class',
  interface: 'interface',
  enum: 'enumeration',
  annotation: 'annotation',
}

/**
 * Relationships are distinguished by line style and arrowhead, following UML,
 * rather than by hue — a second colour scale would collide with the package
 * one, and the three kinds need to stay readable on top of it.
 */
export interface RelationStyle {
  label: string
  /** SVG dash pattern; `undefined` draws a solid line. */
  dash?: string
  /** Closed triangle for inheritance, open arrow for association. */
  closedArrow: boolean
  width: number
}

export const RELATION_STYLES: Record<RelationKind, RelationStyle> = {
  extends: { label: 'extends', closedArrow: true, width: 1.75 },
  implements: {
    label: 'implements',
    dash: '7 5',
    closedArrow: true,
    width: 1.75,
  },
  association: {
    label: 'association',
    dash: '2 4',
    closedArrow: false,
    width: 1.25,
  },
}

/** Edges stay recessive so the nodes carry the colour. */
export const EDGE_COLOR = 'oklch(0.58 0 0)'
export const EDGE_COLOR_HIGHLIGHT = 'oklch(0.95 0 0)'
export const EDGE_COLOR_DIMMED = 'oklch(0.32 0 0)'
