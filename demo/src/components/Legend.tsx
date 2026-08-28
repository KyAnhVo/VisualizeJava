const KINDS = [
  { label: 'class', color: 'var(--kind-class)' },
  { label: 'interface', color: 'var(--kind-interface)' },
  { label: 'enum', color: 'var(--kind-enum)' },
  { label: '@interface', color: 'var(--kind-annotation)' },
] as const

export function Legend({ mode }: { mode: 'types' | 'packages' }) {
  return (
    <div className="pointer-events-none select-none rounded-lg border border-border bg-card/85 p-3 text-[11px] backdrop-blur">
      <div className="mb-2 font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
        edges
      </div>

      <div className="space-y-1.5">
        {mode === 'packages' ? (
          <>
            {/*
              Only 5 of prod's 37 package edges are inheritance, so an
              inheritance-only package view would be nearly empty. Both relations
              are drawn up here, and thickness carries the weight.
            */}
            <LegendEdge
              color="var(--edge-extends)"
              width={3.4}
              label="contains an inheritance link"
            />
            <LegendEdge
              color="var(--edge-assoc)"
              width={2}
              label="references only"
            />
            <div className="text-muted-foreground">
              thickness &amp; number = type pairs involved
            </div>
          </>
        ) : (
          <>
            <LegendEdge
              color="var(--edge-extends)"
              width={3.4}
              label="extends — class inherits class"
            />
            <LegendEdge
              color="var(--edge-implements)"
              width={1.8}
              dashed
              label="implements — and interface extends"
            />
            <LegendEdge
              color="var(--edge-assoc)"
              width={2}
              label="association — shown on selection"
            />
          </>
        )}
      </div>

      <div className="mt-3 mb-2 font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
        types
      </div>
      <div className="flex flex-wrap gap-x-3 gap-y-1 font-mono">
        {KINDS.map((kind) => (
          <span key={kind.label} className="flex items-center gap-1.5">
            <span
              className="h-2 w-2 rounded-[2px]"
              style={{ background: kind.color }}
            />
            <span className="text-muted-foreground">{kind.label}</span>
          </span>
        ))}
      </div>
    </div>
  )
}

function LegendEdge({
  color,
  width,
  dashed,
  label,
}: {
  color: string
  width: number
  dashed?: boolean
  label: string
}) {
  return (
    <div className="flex items-center gap-2">
      <svg width="34" height="10" aria-hidden className="shrink-0">
        <line
          x1="0"
          y1="5"
          x2="26"
          y2="5"
          stroke={color}
          strokeWidth={width}
          strokeDasharray={dashed ? '5 4' : undefined}
        />
        <path d="M26 1 L33 5 L26 9 Z" fill={dashed ? 'none' : color} stroke={color} />
      </svg>
      <span className="text-muted-foreground">{label}</span>
    </div>
  )
}
