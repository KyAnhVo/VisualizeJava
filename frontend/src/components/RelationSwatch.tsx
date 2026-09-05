import { RELATION_STYLES } from '@/lib/palette'
import type { RelationKind } from '@/types/graph'

/**
 * Miniature of the real edge: same dash pattern and arrowhead. Relationship
 * kinds are encoded by line style rather than hue, so the swatch is the legend.
 */
export function RelationSwatch({
  kind,
  color = 'currentColor',
}: {
  kind: RelationKind
  color?: string
}) {
  const style = RELATION_STYLES[kind]
  const markerId = `swatch-arrow-${kind}-${style.closedArrow ? 'closed' : 'open'}`

  return (
    <svg width="26" height="10" viewBox="0 0 26 10" aria-hidden focusable="false">
      <defs>
        <marker
          id={markerId}
          markerWidth="7"
          markerHeight="7"
          refX="6"
          refY="3.5"
          orient="auto"
        >
          {style.closedArrow ? (
            <path d="M0,0.5 L6.5,3.5 L0,6.5 Z" fill={color} />
          ) : (
            <path
              d="M0.5,0.75 L6,3.5 L0.5,6.25"
              fill="none"
              stroke={color}
              strokeWidth="1.2"
            />
          )}
        </marker>
      </defs>
      <line
        x1="1"
        y1="5"
        x2="18"
        y2="5"
        stroke={color}
        strokeWidth={style.width}
        strokeDasharray={style.dash}
        markerEnd={`url(#${markerId})`}
      />
    </svg>
  )
}
