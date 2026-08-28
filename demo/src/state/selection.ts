import { memberSlot, type AssociationModel, type GraphModel } from '@/model/build'

export type Selection =
  | { kind: 'none' }
  | { kind: 'type'; typeKey: string }
  | { kind: 'member'; typeKey: string; memberKey: string }

export const NO_SELECTION: Selection = { kind: 'none' }

/**
 * One association edge on the canvas, aggregated to the type level.
 *
 * Members live in the inspector, not in the node boxes, so an edge can no longer
 * anchor on a member row — and it no longer needs to. Aggregating collapses the
 * worst case dramatically: in `test_target/prod`, clicking `java.nio.IntBuffer`
 * used to mean 212 member-anchored edges (and force-opening a 494-member type);
 * as distinct owner types it is 19. The members behind the edge are not lost —
 * they are listed in `memberKeys` and shown in the inspector.
 */
export interface AssocLink {
  id: string
  ownerKey: string
  targetKey: string
  /** The members of `ownerKey` that produce this link. Never empty. */
  memberKeys: string[]
}

export interface Highlight {
  active: boolean
  /** The association edges to draw. Empty unless a selection is active. */
  links: AssocLink[]
  /** Types that stay at full opacity. */
  litTypes: Set<string>
  /** The type the user actually clicked, if any. */
  anchorKey: string | null
  /** The member the user clicked, if the selection is a member. */
  anchorMemberKey: string | null
}

const EMPTY: Highlight = {
  active: false,
  links: [],
  litTypes: new Set(),
  anchorKey: null,
  anchorMemberKey: null,
}

/**
 * Turns a click into everything the canvas needs to render.
 *
 * The two directions of the spec are duals of the same association triple:
 * clicking a *member* asks "what does this member point at", clicking a *type*
 * asks "which members point at me".
 */
export function deriveHighlight(
  selection: Selection,
  model: GraphModel,
): Highlight {
  if (selection.kind === 'none') return EMPTY

  const associations =
    selection.kind === 'member'
      ? (model.byMember.get(
          memberSlot(selection.typeKey, selection.memberKey),
        ) ?? [])
      : (model.byTarget.get(selection.typeKey) ?? [])

  const links = aggregate(associations)

  const litTypes = new Set<string>([selection.typeKey])
  for (const link of links) {
    litTypes.add(link.ownerKey)
    litTypes.add(link.targetKey)
  }

  // Enclosing types stay lit: a nested type is drawn *inside* its parent, so
  // dimming the parent would dim the child along with it.
  for (const key of [...litTypes]) {
    let cursor = model.types.get(key)?.parentKey ?? null
    while (cursor) {
      litTypes.add(cursor)
      cursor = model.types.get(cursor)?.parentKey ?? null
    }
  }

  return {
    active: true,
    links,
    litTypes,
    anchorKey: selection.typeKey,
    anchorMemberKey: selection.kind === 'member' ? selection.memberKey : null,
  }
}

/** Collapses `(owner, member, target)` triples to one edge per `(owner, target)`. */
function aggregate(associations: AssociationModel[]): AssocLink[] {
  const links = new Map<string, AssocLink>()
  for (const assoc of associations) {
    const id = `link:${assoc.ownerKey}->${assoc.targetKey}`
    const existing = links.get(id)
    if (existing) existing.memberKeys.push(assoc.memberKey)
    else
      links.set(id, {
        id,
        ownerKey: assoc.ownerKey,
        targetKey: assoc.targetKey,
        memberKeys: [assoc.memberKey],
      })
  }
  return [...links.values()]
}

/** Distinct types on the far side of `associations`, for the navigator. */
export function distinctTypes(
  associations: AssociationModel[],
  side: 'ownerKey' | 'targetKey',
): string[] {
  return [...new Set(associations.map((a) => a[side]))]
}

export function isSameSelection(a: Selection, b: Selection): boolean {
  if (a.kind !== b.kind) return false
  if (a.kind === 'none' || b.kind === 'none') return true
  if (a.typeKey !== (b as { typeKey: string }).typeKey) return false
  if (a.kind === 'member' && b.kind === 'member') {
    return a.memberKey === b.memberKey
  }
  return true
}
