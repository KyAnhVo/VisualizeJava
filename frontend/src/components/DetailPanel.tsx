import { useMemo } from 'react'
import { Crosshair, X } from 'lucide-react'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Separator } from '@/components/ui/separator'
import {
  accessGlyph,
  displayModifiers,
  formatMemberSignature,
  isConstructor,
  memberCategory,
  simpleName,
} from '@/lib/java'
import { RELATION_STYLES, VARIANT_LABEL } from '@/lib/palette'
import type { ProjectGraph, ProjectRelation, ProjectType, RelationKind } from '@/types/graph'
import type { Member } from '@/types/wasm-graph'

interface DetailPanelProps {
  type: ProjectType
  graph: ProjectGraph
  isolatedId: string | null
  onSelect: (id: string) => void
  onToggleIsolate: () => void
  onClose: () => void
}

interface RelatedEntry {
  id: string
  kind: RelationKind
  via: string[]
}

function collectRelated(relations: ProjectRelation[], id: string) {
  const outgoing: RelatedEntry[] = []
  const incoming: RelatedEntry[] = []
  for (const relation of relations) {
    if (relation.source === id) {
      outgoing.push({ id: relation.target, kind: relation.kind, via: relation.via })
    }
    if (relation.target === id) {
      incoming.push({ id: relation.source, kind: relation.kind, via: relation.via })
    }
  }
  return { outgoing, incoming }
}

function MemberRow({ member }: { member: Member }) {
  const modifiers = displayModifiers(member)
  return (
    <li className="flex items-baseline gap-2 py-1 font-mono text-xs leading-relaxed">
      <span
        className="w-2 shrink-0 text-muted-foreground"
        title={member.modifiers.access_modifier}
      >
        {accessGlyph(member.modifiers.access_modifier)}
      </span>
      <span className="min-w-0 break-words text-foreground/90">
        {formatMemberSignature(member)}
      </span>
      {modifiers.length > 0 && (
        <span className="shrink-0 text-[10px] text-muted-foreground/70">
          {modifiers.join(' ')}
        </span>
      )}
    </li>
  )
}

function Section({
  title,
  count,
  children,
}: {
  title: string
  count: number
  children: React.ReactNode
}) {
  if (count === 0) return null
  return (
    <section className="space-y-1">
      <h3 className="text-[11px] font-medium tracking-wide text-muted-foreground uppercase">
        {title} <span className="tabular-nums opacity-60">{count}</span>
      </h3>
      {children}
    </section>
  )
}

function RelatedList({
  title,
  entries,
  onSelect,
}: {
  title: string
  entries: RelatedEntry[]
  onSelect: (id: string) => void
}) {
  if (entries.length === 0) return null
  return (
    <section className="space-y-1">
      <h3 className="text-[11px] font-medium tracking-wide text-muted-foreground uppercase">
        {title} <span className="tabular-nums opacity-60">{entries.length}</span>
      </h3>
      <ul>
        {entries.map((entry) => (
          <li key={`${entry.kind}-${entry.id}`}>
            <button
              type="button"
              onClick={() => onSelect(entry.id)}
              className="flex w-full items-baseline gap-2 rounded px-1 py-1 text-left text-xs hover:bg-accent"
              title={entry.id}
            >
              <span className="w-[68px] shrink-0 text-[10px] text-muted-foreground">
                {RELATION_STYLES[entry.kind].label}
              </span>
              <span className="min-w-0 truncate text-foreground/90">
                {simpleName(entry.id)}
              </span>
              {entry.via.length > 0 && (
                <span className="ml-auto shrink-0 truncate pl-2 font-mono text-[10px] text-muted-foreground/70">
                  {entry.via.join(', ')}
                </span>
              )}
            </button>
          </li>
        ))}
      </ul>
    </section>
  )
}

export function DetailPanel({
  type,
  graph,
  isolatedId,
  onSelect,
  onToggleIsolate,
  onClose,
}: DetailPanelProps) {
  // Relations are read from the full graph, not the filtered view: the panel
  // should answer "what is this type connected to", not "what is on screen".
  const { outgoing, incoming } = useMemo(
    () => collectRelated(graph.relations, type.id),
    [graph.relations, type.id],
  )

  const fields = type.members.filter((m) => memberCategory(m) === 'field')
  const constructors = type.members.filter(isConstructor)
  const methods = type.members.filter(
    (m) => memberCategory(m) === 'method' && !isConstructor(m),
  )

  return (
    <aside className="flex h-full w-[22rem] shrink-0 flex-col border-l border-border bg-background">
      <header className="flex items-start gap-2 p-4 pb-3">
        <div className="min-w-0 flex-1">
          <div className="text-[11px] text-muted-foreground">
            «{VARIANT_LABEL[type.variant]}»
          </div>
          <h2 className="truncate text-base font-semibold" title={type.id}>
            {type.simpleName}
          </h2>
          <div className="truncate text-xs text-muted-foreground">
            {type.packageName || '(default package)'}
          </div>
        </div>
        <Button
          variant={isolatedId === type.id ? 'secondary' : 'ghost'}
          size="icon"
          onClick={onToggleIsolate}
          title={
            isolatedId === type.id
              ? 'Show the whole graph again'
              : 'Dim everything except direct neighbours'
          }
        >
          <Crosshair className="size-4" aria-hidden />
          <span className="sr-only">Toggle neighbourhood isolation</span>
        </Button>
        <Button variant="ghost" size="icon" onClick={onClose} title="Close">
          <X className="size-4" aria-hidden />
          <span className="sr-only">Close details</span>
        </Button>
      </header>

      <Separator />

      <ScrollArea className="min-h-0 flex-1">
        <div className="space-y-5 p-4">
          <Section title="Constants" count={type.enumValues.length}>
            <div className="flex flex-wrap gap-1">
              {type.enumValues.map((value) => (
                <Badge key={value} variant="secondary" className="font-mono text-[10px]">
                  {value}
                </Badge>
              ))}
            </div>
          </Section>

          <Section title="Fields" count={fields.length}>
            <ul>
              {fields.map((member, index) => (
                <MemberRow key={`${member.name}-${index}`} member={member} />
              ))}
            </ul>
          </Section>

          <Section title="Constructors" count={constructors.length}>
            <ul>
              {constructors.map((member, index) => (
                <MemberRow key={`${member.name}-${index}`} member={member} />
              ))}
            </ul>
          </Section>

          <Section title="Methods" count={methods.length}>
            <ul>
              {methods.map((member, index) => (
                <MemberRow key={`${member.name}-${index}`} member={member} />
              ))}
            </ul>
          </Section>

          {type.members.length === 0 && type.enumValues.length === 0 && (
            <p className="text-xs text-muted-foreground">
              No members were parsed for this type.
            </p>
          )}

          <Separator />

          <RelatedList title="Depends on" entries={outgoing} onSelect={onSelect} />
          <RelatedList title="Used by" entries={incoming} onSelect={onSelect} />
          {outgoing.length === 0 && incoming.length === 0 && (
            <p className="text-xs text-muted-foreground">
              No relationships to other types in this project.
            </p>
          )}
        </div>
      </ScrollArea>
    </aside>
  )
}
