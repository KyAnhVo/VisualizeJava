import { Handle, Position } from '@xyflow/react'
import { memo } from 'react'

import {
  HANDLE_ASSOC_LEFT_SOURCE,
  HANDLE_ASSOC_LEFT_TARGET,
  HANDLE_ASSOC_RIGHT_SOURCE,
  HANDLE_ASSOC_RIGHT_TARGET,
  HANDLE_INHERIT_SOURCE,
  HANDLE_INHERIT_TARGET,
} from '@/flow/handles'
import {
  CHILD_LABEL_H,
  ENUM_GAP,
  ENUM_LABEL_H,
  ENUM_ROW_H,
  NODE_PAD_X,
  summaryText,
  type TypeGeometry,
} from '@/flow/geometry'
import { cn } from '@/lib/utils'
import type { TypeKind, TypeModel } from '@/model/build'

export interface TypeNodeData extends Record<string, unknown> {
  type: TypeModel
  geometry: TypeGeometry
  dimmed: boolean
  selected: boolean
  /** The current selection reaches this type through an association edge. */
  linked: boolean
  /** This type is what the focus level is centred on. */
  focused: boolean
  /** Neighbours the current view left out — reported so the boundary is honest. */
  hiddenNeighbours: number
  onSelectType: (typeKey: string) => void
  onExpand: (typeKey: string) => void
}

const KIND_ACCENT: Record<TypeKind, string> = {
  class: 'var(--kind-class)',
  interface: 'var(--kind-interface)',
  enum: 'var(--kind-enum)',
  annotation: 'var(--kind-annotation)',
}

const KIND_LABEL: Record<TypeKind, string> = {
  class: '«class»',
  interface: '«interface»',
  enum: '«enum»',
  annotation: '«@interface»',
}

function TypeNodeImpl({ data }: { data: TypeNodeData }) {
  const {
    type,
    geometry,
    dimmed,
    selected,
    linked,
    focused,
    hiddenNeighbours,
    onSelectType,
    onExpand,
  } = data
  const accent = KIND_ACCENT[type.kind]
  const handleY = geometry.headerHeight / 2

  return (
    <div
      className={cn(
        'vj-node vj-fade relative flex h-full w-full flex-col overflow-hidden rounded-lg border bg-card font-mono',
        'shadow-[0_1px_0_0_rgba(255,255,255,0.04)_inset,0_8px_24px_-12px_rgba(0,0,0,0.9)]',
        dimmed && 'vj-dim',
      )}
      style={{
        borderColor: selected || linked || focused ? accent : 'var(--border)',
        boxShadow:
          selected || focused
            ? `0 0 0 ${focused ? 3 : 2}px color-mix(in oklch, ${accent} ${focused ? 75 : 60}%, transparent)`
            : undefined,
        // Drives the far-zoom tier, where the box itself has to carry the kind.
        ['--vj-accent' as string]: accent,
      }}
      onClick={() => onSelectType(type.key)}
    >
      {/* A hairline in the type's colour makes the stereotype readable at a glance. */}
      <div
        className="vj-lod-near absolute inset-x-0 top-0 h-[2px]"
        style={{ background: accent }}
      />

      <Handle type="source" position={Position.Top} id={HANDLE_INHERIT_SOURCE} isConnectable={false} />
      <Handle type="target" position={Position.Bottom} id={HANDLE_INHERIT_TARGET} isConnectable={false} />
      <Handle type="source" position={Position.Left} id={HANDLE_ASSOC_LEFT_SOURCE} isConnectable={false} style={{ top: handleY }} />
      <Handle type="target" position={Position.Left} id={HANDLE_ASSOC_LEFT_TARGET} isConnectable={false} style={{ top: handleY }} />
      <Handle type="source" position={Position.Right} id={HANDLE_ASSOC_RIGHT_SOURCE} isConnectable={false} style={{ top: handleY }} />
      <Handle type="target" position={Position.Right} id={HANDLE_ASSOC_RIGHT_TARGET} isConnectable={false} style={{ top: handleY }} />

      <header
        className="vj-lod-near flex shrink-0 flex-col justify-center"
        style={{ height: geometry.headerHeight, paddingInline: NODE_PAD_X }}
      >
        <div className="flex h-[18px] items-center gap-[6px]">
          <span
            className="shrink-0 rounded-[3px] px-[6px] text-[11px] leading-[16px]"
            style={{
              color: accent,
              background: `color-mix(in oklch, ${accent} 16%, transparent)`,
            }}
          >
            {KIND_LABEL[type.kind]}
          </span>
          <span className="truncate text-[13px] font-semibold text-foreground">
            {type.displayName}
          </span>
        </div>
        {!geometry.compact && (
          <div className="mt-[2px] h-[12px] truncate text-[10px] leading-[12px] text-muted-foreground">
            {type.packageName || '(default package)'}
          </div>
        )}
      </header>

      {type.enumValues.length > 0 && (
        <section
          className="vj-lod-near shrink-0 border-t border-border/60"
          style={{
            height: geometry.enumHeight,
            paddingInline: NODE_PAD_X,
            paddingBlock: 6,
          }}
        >
          <div
            className="text-[10px] uppercase tracking-wider text-muted-foreground"
            style={{ height: ENUM_LABEL_H, lineHeight: `${ENUM_LABEL_H}px` }}
          >
            values
          </div>
          <div className="flex flex-wrap" style={{ gap: ENUM_GAP }}>
            {type.enumValues.map((value) => (
              <span
                key={value}
                className="rounded-[3px] px-[7px] text-[11px]"
                style={{
                  height: ENUM_ROW_H,
                  lineHeight: `${ENUM_ROW_H}px`,
                  color: 'var(--kind-enum)',
                  background:
                    'color-mix(in oklch, var(--kind-enum) 14%, transparent)',
                }}
              >
                {value}
              </span>
            ))}
          </div>
        </section>
      )}

      {!geometry.compact && (
        <div
          className="vj-lod-near flex shrink-0 items-center gap-[4px] border-t border-border/60 text-[11px] text-muted-foreground"
          style={{ height: geometry.summaryHeight, paddingInline: NODE_PAD_X }}
        >
          <span className="truncate">{summaryText(type)}</span>
        </div>
      )}

      {type.childKeys.length > 0 && (
        <>
          <div
            className="vj-lod-near shrink-0 border-t border-dashed border-border/60 text-[10px] uppercase tracking-wider text-muted-foreground"
            style={{
              height: CHILD_LABEL_H,
              lineHeight: `${CHILD_LABEL_H}px`,
              paddingInline: NODE_PAD_X,
            }}
          >
            nested types
          </div>
          {/* The remaining space is the well ELK packed the children into. */}
          <div className="flex-1 bg-[color-mix(in_oklch,var(--background)_55%,transparent)]" />
        </>
      )}

      {/*
        A focus view is a claim about a neighbourhood, and a silent boundary
        would read as "this type has no other relationships". The badge says how
        many were left out and re-centres on this type when clicked.
      */}
      {hiddenNeighbours > 0 && (
        <button
          type="button"
          title={`${hiddenNeighbours} more related ${hiddenNeighbours === 1 ? 'type' : 'types'} — click to centre here`}
          onClick={(event) => {
            event.stopPropagation()
            onExpand(type.key)
          }}
          className="vj-lod-near absolute -right-[7px] -top-[7px] z-10 rounded-full border border-border bg-card px-[5px] text-[10px] leading-[15px] text-muted-foreground transition-colors hover:border-ring hover:text-foreground"
        >
          +{hiddenNeighbours}
        </button>
      )}

      {/*
        Below ~0.5 zoom the 11px text renders at 5px or less, so the card is
        replaced by the name alone at a size that survives the transform — and
        below ~0.28 by a solid block in the kind's colour. 257 boxes cannot all
        be labelled legibly on one screen; at that point the useful signal is the
        inheritance skeleton, and the inspector and search are how you read names.
      */}
      <div className="vj-lod-mid absolute inset-0 hidden place-items-center px-[6px] text-center">
        <span
          className="w-full truncate text-[24px] font-semibold leading-tight"
          style={{ color: accent }}
        >
          {type.simpleName}
        </span>
      </div>
    </div>
  )
}

export const TypeNode = memo(TypeNodeImpl)
