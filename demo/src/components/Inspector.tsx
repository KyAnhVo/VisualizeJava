import { ArrowLeftToLine, ArrowRightToLine, Crosshair, X } from 'lucide-react'
import { useEffect, useMemo, useRef } from 'react'

import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'
import { memberSlot, type GraphModel, type TypeKind, type TypeModel } from '@/model/build'
import { distinctTypes, type Selection } from '@/state/selection'

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

const CATEGORY_LABEL = {
  field: 'fields',
  constructor: 'constructors',
  method: 'methods',
} as const

/**
 * The members panel, and the navigator beside it.
 *
 * Members are deliberately *not* drawn inside the canvas nodes. A real project
 * has types with hundreds of them (`test_target/prod` peaks at 494), and putting
 * them in the box made node height a function of selection: opening one 494-row
 * type produced a ~10,000px node and forced a full ELK relayout. Here the list
 * is just a scroll container, so it costs nothing and the canvas geometry is
 * fixed.
 *
 * The navigator is the other half of that trade. Association edges are drawn
 * type-to-type, so this is where you see *which* members are behind an edge and
 * jump to the type on the far side of it.
 */
export function Inspector({
  model,
  selection,
  inView,
  onSelectMember,
  onJump,
  onClose,
}: {
  model: GraphModel
  selection: Selection
  /** The slice currently on canvas, or null at the package overview. */
  inView: GraphModel | null
  onSelectMember: (typeKey: string, memberKey: string) => void
  onJump: (typeKey: string) => void
  onClose: () => void
}) {
  const type = selection.kind === 'none' ? null : model.types.get(selection.typeKey)
  if (!type) return null

  const memberKey = selection.kind === 'member' ? selection.memberKey : null

  return (
    <aside className="pointer-events-auto absolute inset-y-0 right-0 flex max-w-full">
      <Navigator
        model={model}
        inView={inView}
        type={type}
        memberKey={memberKey}
        onJump={onJump}
      />
      <MemberPanel
        model={model}
        type={type}
        selectedMemberKey={memberKey}
        onSelectMember={onSelectMember}
        onClose={onClose}
      />
    </aside>
  )
}

function Navigator({
  model,
  inView,
  type,
  memberKey,
  onJump,
}: {
  model: GraphModel
  inView: GraphModel | null
  type: TypeModel
  memberKey: string | null
  onJump: (typeKey: string) => void
}) {
  const groups = useMemo(() => {
    if (memberKey) {
      const targets = model.byMember.get(memberSlot(type.key, memberKey)) ?? []
      return [
        {
          id: 'refs',
          label: 'references',
          hint: 'types this member uses',
          icon: <ArrowRightToLine className="size-3" />,
          keys: distinctTypes(targets, 'targetKey'),
        },
      ]
    }
    return [
      {
        id: 'in',
        label: 'used by',
        hint: 'types whose members refer to this one',
        icon: <ArrowLeftToLine className="size-3" />,
        keys: distinctTypes(model.byTarget.get(type.key) ?? [], 'ownerKey').filter(
          (key) => key !== type.key,
        ),
      },
      {
        id: 'out',
        label: 'uses',
        hint: 'types this one refers to',
        icon: <ArrowRightToLine className="size-3" />,
        keys: distinctTypes(model.byOwner.get(type.key) ?? [], 'targetKey').filter(
          (key) => key !== type.key,
        ),
      },
    ]
  }, [model, type, memberKey])

  return (
    <nav className="flex w-52 shrink-0 flex-col overflow-y-auto border-l border-border bg-background/95 backdrop-blur">
      {groups.map((group) => (
        <section key={group.id} className="border-b border-border/60 p-2">
          <div
            className="mb-1.5 flex items-center gap-1.5 font-mono text-[10px] uppercase tracking-wider text-muted-foreground"
            title={group.hint}
          >
            {group.icon}
            {group.label}
            <span className="ml-auto tabular-nums">{group.keys.length}</span>
          </div>

          {group.keys.length === 0 ? (
            <div className="px-1 py-1 font-mono text-[11px] italic text-muted-foreground/70">
              none
            </div>
          ) : (
            <ul className="space-y-px">
              {group.keys
                .map((key) => model.types.get(key))
                .filter((t): t is TypeModel => Boolean(t))
                .sort((a, b) => a.displayName.localeCompare(b.displayName))
                .map((target) => {
                  // The navigator reports the whole graph, not just what is
                  // drawn — that is how you find where to go next — so it has to
                  // say which entries are off-canvas.
                  const offCanvas = inView !== null && !inView.types.has(target.key)
                  return (
                    <li key={target.key}>
                      <button
                        type="button"
                        onClick={() => onJump(target.key)}
                        title={
                          offCanvas
                            ? `${target.key} — not in this view; click to focus it`
                            : target.key
                        }
                        className="group flex w-full items-center gap-1.5 rounded px-1.5 py-1 text-left font-mono hover:bg-muted"
                      >
                        <span
                          className={cn(
                            'size-1.5 shrink-0 rounded-[1px]',
                            offCanvas && 'opacity-40',
                          )}
                          style={{ background: KIND_ACCENT[target.kind] }}
                        />
                        <span className="min-w-0 flex-1">
                          <span
                            className={cn(
                              'block truncate text-[11px]',
                              offCanvas ? 'text-muted-foreground' : 'text-foreground',
                            )}
                          >
                            {target.displayName}
                          </span>
                          <span className="block truncate text-[9px] text-muted-foreground">
                            {target.packageName}
                          </span>
                        </span>
                        <Crosshair className="size-3 shrink-0 text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100" />
                      </button>
                    </li>
                  )
                })}
            </ul>
          )}
        </section>
      ))}
    </nav>
  )
}

function MemberPanel({
  model,
  type,
  selectedMemberKey,
  onSelectMember,
  onClose,
}: {
  model: GraphModel
  type: TypeModel
  selectedMemberKey: string | null
  onSelectMember: (typeKey: string, memberKey: string) => void
  onClose: () => void
}) {
  const accent = KIND_ACCENT[type.kind]
  const scroller = useRef<HTMLDivElement>(null)
  const selectedRow = useRef<HTMLLIElement>(null)

  // Selecting a member from the canvas or the navigator can target a row far
  // down a 494-entry list, so bring it into view rather than leaving the panel
  // apparently unchanged.
  useEffect(() => {
    if (selectedMemberKey) {
      selectedRow.current?.scrollIntoView({ block: 'nearest' })
    } else {
      scroller.current?.scrollTo({ top: 0 })
    }
  }, [selectedMemberKey, type.key])

  const groups = (['field', 'constructor', 'method'] as const)
    .map((category) => ({
      category,
      members: type.members.filter((m) => m.category === category),
    }))
    .filter((group) => group.members.length > 0)

  return (
    <div className="flex w-96 shrink-0 flex-col border-l border-border bg-card/95 backdrop-blur">
      <header className="shrink-0 border-b border-border p-3">
        <div className="flex items-start gap-2">
          <div className="min-w-0 flex-1">
            <div className="flex items-center gap-1.5">
              <span
                className="shrink-0 rounded-[3px] px-1.5 font-mono text-[10px] leading-4"
                style={{
                  color: accent,
                  background: `color-mix(in oklch, ${accent} 16%, transparent)`,
                }}
              >
                {KIND_LABEL[type.kind]}
              </span>
              <span className="truncate font-mono text-sm font-semibold">
                {type.displayName}
              </span>
            </div>
            <div className="mt-0.5 truncate font-mono text-[10px] text-muted-foreground">
              {type.packageName || '(default package)'}
            </div>
          </div>
          <Button size="icon-xs" variant="ghost" onClick={onClose} aria-label="Close inspector">
            <X />
          </Button>
        </div>

        {type.enumValues.length > 0 && (
          <div className="mt-2 flex flex-wrap gap-1">
            {type.enumValues.map((value) => (
              <span
                key={value}
                className="rounded-[3px] px-1.5 py-0.5 font-mono text-[10px]"
                style={{
                  color: 'var(--kind-enum)',
                  background: 'color-mix(in oklch, var(--kind-enum) 14%, transparent)',
                }}
              >
                {value}
              </span>
            ))}
          </div>
        )}
      </header>

      <div ref={scroller} className="min-h-0 flex-1 overflow-y-auto">
        {type.members.length === 0 && (
          <p className="p-3 font-mono text-[11px] italic text-muted-foreground">
            no members
          </p>
        )}

        {groups.map((group) => (
          <section key={group.category}>
            <h3 className="sticky top-0 z-10 border-b border-border/60 bg-card/95 px-3 py-1 font-mono text-[10px] uppercase tracking-wider text-muted-foreground backdrop-blur">
              {CATEGORY_LABEL[group.category]}
              <span className="ml-1.5 tabular-nums opacity-60">
                {group.members.length}
              </span>
            </h3>
            <ul>
              {group.members.map((member) => {
                const refs =
                  model.byMember.get(memberSlot(type.key, member.key))?.length ?? 0
                const selected = member.key === selectedMemberKey
                return (
                  <li key={member.key} ref={selected ? selectedRow : undefined}>
                    <button
                      type="button"
                      onClick={() => onSelectMember(type.key, member.key)}
                      className={cn(
                        'flex w-full items-start gap-2 px-3 py-1 text-left font-mono text-[11px]',
                        selected
                          ? 'bg-[color-mix(in_oklch,var(--edge-assoc)_18%,transparent)] text-foreground'
                          : 'text-muted-foreground hover:bg-muted/60 hover:text-foreground',
                      )}
                    >
                      <span
                        className="w-2 shrink-0 text-center"
                        style={{ color: selected ? 'var(--edge-assoc)' : undefined }}
                      >
                        {member.glyph}
                      </span>
                      <span className="min-w-0 flex-1">
                        {member.annotations.map((annotation) => (
                          <span
                            key={annotation}
                            className="block truncate text-[10px]"
                            style={{ color: 'var(--kind-annotation)' }}
                          >
                            {annotation}
                          </span>
                        ))}
                        <span
                          className={cn(
                            'block break-words',
                            member.modifiers.includes('static') && 'italic',
                          )}
                        >
                          {member.signature}
                        </span>
                        {member.modifiers.length > 0 && (
                          <span className="block text-[9px] uppercase tracking-wider opacity-60">
                            {member.modifiers.join(' ')}
                          </span>
                        )}
                      </span>
                      {refs > 0 && (
                        <span
                          className="mt-px shrink-0 rounded-[3px] px-1 text-[9px] leading-4 tabular-nums"
                          style={{
                            color: 'var(--edge-assoc)',
                            background:
                              'color-mix(in oklch, var(--edge-assoc) 14%, transparent)',
                          }}
                          title={`refers to ${refs} project ${refs === 1 ? 'type' : 'types'}`}
                        >
                          →{refs}
                        </span>
                      )}
                    </button>
                  </li>
                )
              })}
            </ul>
          </section>
        ))}
      </div>
    </div>
  )
}
