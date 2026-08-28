import { Handle, Position } from '@xyflow/react'
import { memo } from 'react'

import {
  PKG_COUNTS_H,
  PKG_NAME_H,
  PKG_PAD_X,
  PKG_PAD_Y,
  PKG_PREFIX_H,
  type PackageGeometry,
} from '@/flow/geometry'
import {
  HANDLE_ASSOC_LEFT_SOURCE,
  HANDLE_ASSOC_LEFT_TARGET,
  HANDLE_ASSOC_RIGHT_SOURCE,
  HANDLE_ASSOC_RIGHT_TARGET,
  HANDLE_INHERIT_SOURCE,
  HANDLE_INHERIT_TARGET,
} from '@/flow/handles'
import { cn } from '@/lib/utils'
import type { TypeKind } from '@/model/build'
import { packageKinds, type PackageModel } from '@/model/packages'

export interface PackageNodeData extends Record<string, unknown> {
  pkg: PackageModel
  geometry: PackageGeometry
  dimmed: boolean
  onOpen: (packageName: string) => void
}

const KIND_ACCENT: Record<TypeKind, string> = {
  class: 'var(--kind-class)',
  interface: 'var(--kind-interface)',
  enum: 'var(--kind-enum)',
  annotation: 'var(--kind-annotation)',
}

function PackageNodeImpl({ data }: { data: PackageNodeData }) {
  const { pkg, geometry, dimmed, onOpen } = data

  return (
    <div
      className={cn(
        'vj-fade relative flex h-full w-full cursor-pointer flex-col justify-center rounded-lg border border-border bg-card font-mono',
        'shadow-[0_1px_0_0_rgba(255,255,255,0.04)_inset,0_8px_24px_-12px_rgba(0,0,0,0.9)]',
        'transition-colors hover:border-ring',
        dimmed && 'vj-dim',
      )}
      style={{ paddingInline: PKG_PAD_X, paddingBlock: PKG_PAD_Y }}
      onClick={() => onOpen(pkg.name)}
      title={`${pkg.name || '(default package)'} — ${pkg.total} types`}
    >
      <Handle type="source" position={Position.Top} id={HANDLE_INHERIT_SOURCE} isConnectable={false} />
      <Handle type="target" position={Position.Bottom} id={HANDLE_INHERIT_TARGET} isConnectable={false} />
      <Handle type="source" position={Position.Left} id={HANDLE_ASSOC_LEFT_SOURCE} isConnectable={false} />
      <Handle type="target" position={Position.Left} id={HANDLE_ASSOC_LEFT_TARGET} isConnectable={false} />
      <Handle type="source" position={Position.Right} id={HANDLE_ASSOC_RIGHT_SOURCE} isConnectable={false} />
      <Handle type="target" position={Position.Right} id={HANDLE_ASSOC_RIGHT_TARGET} isConnectable={false} />

      {geometry.hasPrefix && (
        <div
          className="truncate text-[10px] text-muted-foreground"
          style={{ height: PKG_PREFIX_H, lineHeight: `${PKG_PREFIX_H}px` }}
        >
          {pkg.prefix}.
        </div>
      )}

      <div
        className="truncate text-[15px] font-semibold text-foreground"
        style={{ height: PKG_NAME_H, lineHeight: `${PKG_NAME_H}px` }}
      >
        {pkg.leaf}
      </div>

      <div
        className="flex items-center gap-[10px] text-[11px] text-muted-foreground"
        style={{ height: PKG_COUNTS_H }}
      >
        {packageKinds(pkg).map(({ kind, count }) => (
          <span key={kind} className="flex items-center gap-[4px]">
            <span
              className="size-[6px] shrink-0 rounded-[1px]"
              style={{ background: KIND_ACCENT[kind] }}
            />
            {count} {kind}
          </span>
        ))}
      </div>
    </div>
  )
}

export const PackageNode = memo(PackageNodeImpl)
