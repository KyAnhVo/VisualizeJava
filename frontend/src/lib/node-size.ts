import type { ProjectType } from '@/types/graph'
import { VARIANT_LABEL } from './palette'

/**
 * Node geometry is computed, not measured.
 *
 * ELK needs every node's size *before* anything is rendered, and a
 * measure-then-relayout round trip would make the graph visibly jump. The
 * per-character widths below are calibrated to the fonts and sizes used in
 * `TypeNode`; the box is generous enough that a small estimation error only
 * changes padding, never truncates.
 */

export const NODE_HEIGHT = 78

const MIN_WIDTH = 184
const MAX_WIDTH = 340
/** Horizontal padding plus the accent bar. */
const CHROME_WIDTH = 34

const NAME_CHAR_WIDTH = 8.3
const SUBTEXT_CHAR_WIDTH = 6.3
/** Room for the member-count badge sitting beside the stereotype. */
const BADGE_WIDTH = 34

export function nodeWidth(type: ProjectType): number {
  const nameWidth = type.simpleName.length * NAME_CHAR_WIDTH
  const packageWidth = (type.packageName || '(default)').length * SUBTEXT_CHAR_WIDTH
  const stereotypeWidth =
    `«${VARIANT_LABEL[type.variant]}»`.length * SUBTEXT_CHAR_WIDTH + BADGE_WIDTH

  const widest = Math.max(nameWidth, packageWidth, stereotypeWidth)
  return Math.round(
    Math.min(MAX_WIDTH, Math.max(MIN_WIDTH, widest + CHROME_WIDTH)),
  )
}
