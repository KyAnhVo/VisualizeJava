/**
 * Handle ids, kept in their own module so the node component only exports
 * components.
 *
 * Inheritance runs vertically (subtype's top → supertype's bottom, matching
 * ELK's `direction: UP`). Associations run horizontally and need both a source
 * and a target on each side, so an edge can leave from whichever flank faces the
 * other node instead of cutting back across its own box.
 */
export const HANDLE_INHERIT_SOURCE = 'inh-s'
export const HANDLE_INHERIT_TARGET = 'inh-t'

export const HANDLE_ASSOC_LEFT_SOURCE = 'as-l-s'
export const HANDLE_ASSOC_LEFT_TARGET = 'as-l-t'
export const HANDLE_ASSOC_RIGHT_SOURCE = 'as-r-s'
export const HANDLE_ASSOC_RIGHT_TARGET = 'as-r-t'

export function assocHandles(toRight: boolean) {
  return toRight
    ? { source: HANDLE_ASSOC_RIGHT_SOURCE, target: HANDLE_ASSOC_LEFT_TARGET }
    : { source: HANDLE_ASSOC_LEFT_SOURCE, target: HANDLE_ASSOC_RIGHT_TARGET }
}
