import type { TypeModel } from '@/model/build'
import { packageCountsText, type PackageModel } from '@/model/packages'

/**
 * Node geometry is computed, not measured.
 *
 * ELK needs a size for every node *before* anything is rendered, and React Flow
 * positions nodes from that same layout. If the rendered box were a different
 * size than the one ELK reserved, nested types would overflow their parent and
 * edges would land off their anchors. So the sizes here are authoritative and
 * the DOM is pinned to them: every row height below has a matching fixed-height
 * class in `TypeNode`, and all text is monospace so width is predictable.
 *
 * Members are *not* part of node geometry. They live in the inspector panel, so
 * a node's height depends only on its own name, its enum values and whether it
 * is isolated — which makes the layout static for a given graph and keeps a
 * 494-member type the same size as a 2-member one.
 */

/*
 * Advance widths, in px per character.
 *
 * JetBrains Mono is self-hosted (`@fontsource-variable/jetbrains-mono`, bundled
 * by Vite — no CDN), and its advance is exactly 600/1000 em for every glyph and
 * *unchanged* across the weight axis: 400, 600 and 700 all measure 600 units. So
 * the semibold constants use the same ratio as the regular ones.
 *
 * Each is rounded slightly *up* on purpose. Over-estimating reserves a little
 * too much width, which truncates text a character early; under-estimating
 * overflows the box ELK was told about. The margins here are +0.8%, +1.3%, +0.8%
 * and +1.1%. That headroom also covers the fallback stack: the widest common
 * monospace fallback (DejaVu Sans Mono, 0.602 em) still fits.
 */
export const CHAR_W = 6.65 // 11px × 0.6 = 6.60
export const NAME_CHAR_W = 7.9 // 13px × 0.6 = 7.80, semibold
export const PKG_CHAR_W = 6.05 // 10px × 0.6 = 6.00

export const NODE_PAD_X = 10
export const HEADER_H = 48
/** Isolated types drop the package line and the summary row. */
export const COMPACT_H = 30
export const SUMMARY_H = 26
export const ENUM_ROW_H = 18
export const ENUM_GAP = 4
export const ENUM_PAD_Y = 12
export const ENUM_LABEL_H = 14

export const CHILD_PAD = 10
export const CHILD_LABEL_H = 16
export const CHILD_MIN_W = 180
export const CHILD_MIN_H = 60

export const MIN_W = 216
export const MAX_W = 340

export interface TypeGeometry {
  width: number
  /** Height of the type's own content — header, enum values, summary. */
  contentHeight: number
  headerHeight: number
  enumHeight: number
  /** Height of the member-count summary row (0 when compact). */
  summaryHeight: number
  /** Isolated type: single-line header, no package, no summary. */
  compact: boolean
}

export function measureType(type: TypeModel): TypeGeometry {
  const compact = type.isolated
  const width = measureWidth(type, compact)
  const inner = width - 2 * NODE_PAD_X

  // Enum values are shown even on a compact node — the spec asks for all of
  // them, and an isolated enum is exactly the case where nothing else says what
  // the type is.
  const enumHeight = type.enumValues.length
    ? ENUM_PAD_Y + ENUM_LABEL_H + enumChipsHeight(type.enumValues, inner)
    : 0

  const headerHeight = compact ? COMPACT_H : HEADER_H
  const summaryHeight = compact ? 0 : SUMMARY_H

  return {
    width,
    contentHeight: headerHeight + enumHeight + summaryHeight,
    headerHeight,
    enumHeight,
    summaryHeight,
    compact,
  }
}

function measureWidth(type: TypeModel, compact: boolean): number {
  // The header carries a stereotype chip to the left of the name.
  const stereotype = `«${type.kind}»`.length * CHAR_W + 12
  let widest = stereotype + 6 + type.displayName.length * NAME_CHAR_W

  if (!compact) {
    widest = Math.max(widest, type.packageName.length * PKG_CHAR_W)
    widest = Math.max(widest, summaryText(type).length * CHAR_W)
  }

  for (const value of type.enumValues) {
    widest = Math.max(widest, enumChipWidth(value))
  }

  return clamp(Math.ceil(widest + 2 * NODE_PAD_X), MIN_W, MAX_W)
}

export function summaryText(type: TypeModel): string {
  const { fields, constructors, methods } = type.counts
  const parts: string[] = []
  if (fields) parts.push(`${fields} ${plural('field', fields)}`)
  if (constructors) parts.push(`${constructors} ctor${constructors > 1 ? 's' : ''}`)
  if (methods) parts.push(`${methods} ${plural('method', methods)}`)
  return parts.length ? parts.join(' · ') : 'no members'
}

function plural(word: string, count: number): string {
  return count === 1 ? word : `${word}s`
}

export function enumChipWidth(value: string): number {
  return Math.ceil(value.length * CHAR_W) + 14
}

/** Greedy wrap of the enum chips, mirroring what flex-wrap will do. */
export function enumChipsHeight(values: string[], innerWidth: number): number {
  let rows = 1
  let used = 0
  for (const value of values) {
    const w = enumChipWidth(value)
    const needed = used === 0 ? w : used + ENUM_GAP + w
    if (needed > innerWidth && used > 0) {
      rows += 1
      used = w
    } else {
      used = needed
    }
  }
  return rows * ENUM_ROW_H + (rows - 1) * ENUM_GAP
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value))
}

// --- package overview --------------------------------------------------------

export const PKG_PAD_X = 12
export const PKG_PAD_Y = 9
export const PKG_PREFIX_H = 13
export const PKG_NAME_H = 22
export const PKG_COUNTS_H = 18
export const PKG_NAME_CHAR_W = 9.1 // 15px × 0.6 = 9.00, semibold
/** Dot plus its gap, per kind chip in the counts row. */
export const PKG_CHIP_EXTRA = 13
export const PKG_MIN_W = 208
export const PKG_MAX_W = 360

export interface PackageGeometry {
  width: number
  height: number
  /** The default package and single-segment packages have no prefix line. */
  hasPrefix: boolean
}

/**
 * There are only tens of these and they never nest, so the box is fixed: a muted
 * prefix line, the last segment large enough to read at overview zoom, and the
 * kind breakdown.
 */
export function measurePackage(pkg: PackageModel): PackageGeometry {
  const hasPrefix = pkg.prefix.length > 0
  const counts = packageCountsText(pkg)

  let widest = pkg.leaf.length * PKG_NAME_CHAR_W
  widest = Math.max(widest, pkg.prefix.length * PKG_CHAR_W)
  widest = Math.max(
    widest,
    counts.length * CHAR_W + kindChipCount(pkg) * PKG_CHIP_EXTRA,
  )

  return {
    width: clamp(Math.ceil(widest + 2 * PKG_PAD_X), PKG_MIN_W, PKG_MAX_W),
    height:
      2 * PKG_PAD_Y + (hasPrefix ? PKG_PREFIX_H : 0) + PKG_NAME_H + PKG_COUNTS_H,
    hasPrefix,
  }
}

function kindChipCount(pkg: PackageModel): number {
  return Object.values(pkg.counts).filter((count) => count > 0).length
}
