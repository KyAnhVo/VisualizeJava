import { Handle, Position, type NodeProps, type Node } from '@xyflow/react'
import { cn } from '@/lib/utils'
import { VARIANT_LABEL } from '@/lib/palette'
import { NODE_HEIGHT } from '@/lib/node-size'
import type { ProjectType } from '@/types/graph'

export interface TypeNodeData extends Record<string, unknown> {
  type: ProjectType
  color: string
  width: number
  /** Outside the isolated neighbourhood, or filtered out by search. */
  dimmed: boolean
  /** Matches the active search term. */
  highlighted: boolean
}

export type TypeFlowNode = Node<TypeNodeData, 'javaType'>

export const TYPE_NODE_TYPE = 'javaType'

/**
 * Handle ids must be distinct per node. Inheritance leaves a node through its
 * top and arrives at the supertype's bottom (supertypes sit above); an
 * association runs the other way.
 */
export const SOURCE_TOP = 's-top'
export const TARGET_TOP = 't-top'
export const SOURCE_BOTTOM = 's-bottom'
export const TARGET_BOTTOM = 't-bottom'

export function TypeNode({ data, selected }: NodeProps<TypeFlowNode>) {
  const { type, color, width, dimmed, highlighted } = data
  const memberCount = type.members.length
  const count = type.variant === 'enum' ? type.enumValues.length : memberCount
  const countLabel = type.variant === 'enum' ? 'constants' : 'members'

  return (
    <div
      className={cn(
        'relative flex h-full flex-col justify-center overflow-hidden rounded-md border bg-card pr-3 pl-4 text-left transition-opacity',
        selected ? 'border-foreground/70 ring-1 ring-foreground/40' : 'border-border',
        highlighted && !selected && 'border-foreground/40 ring-1 ring-foreground/20',
        dimmed ? 'opacity-25' : 'opacity-100',
      )}
      style={{ width, height: NODE_HEIGHT }}
    >
      {/* Package identity: the accent bar is the colour channel, the package
          name below is the redundant text channel. */}
      <span
        aria-hidden
        className="absolute inset-y-0 left-0 w-1.5"
        style={{ backgroundColor: color }}
      />

      <div className="flex items-baseline justify-between gap-2">
        <span className="truncate text-[11px] leading-tight text-muted-foreground">
          «{VARIANT_LABEL[type.variant]}»
        </span>
        <span
          className="shrink-0 text-[10px] tabular-nums text-muted-foreground/70"
          title={`${count} ${countLabel}`}
        >
          {count}
        </span>
      </div>

      <div className="truncate text-sm leading-snug font-semibold text-foreground">
        {type.simpleName}
      </div>

      <div className="truncate text-[11px] leading-tight text-muted-foreground">
        {type.packageName || '(default package)'}
      </div>

      {/* Four anchors, invisible by design: the graph is a view, not an editor.
          Inheritance is drawn upward (child's top -> parent's bottom) because
          the layout puts supertypes above; association runs the other way. */}
      <Handle type="source" id={SOURCE_TOP} position={Position.Top} />
      <Handle type="target" id={TARGET_TOP} position={Position.Top} />
      <Handle type="source" id={SOURCE_BOTTOM} position={Position.Bottom} />
      <Handle type="target" id={TARGET_BOTTOM} position={Position.Bottom} />
    </div>
  )
}
