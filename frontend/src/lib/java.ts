import type {
  AccessModifier,
  Member,
  RefType,
  TypeArg,
  TypeParam,
  VoidableType,
} from '@/types/wasm-graph'

/** `library.model.Book.Builder` -> `Builder`. */
export function simpleName(qualified: string): string {
  const dot = qualified.lastIndexOf('.')
  return dot === -1 ? qualified : qualified.slice(dot + 1)
}

/**
 * Renders a type reference the way it would read in source: simple name,
 * generic arguments, then array brackets. `extraArrayDims` folds in the
 * C-style dimensions Java allows on the declarator (`int a[]`).
 */
export function formatRefType(ref: RefType, extraArrayDims = 0): string {
  const base = simpleName(ref.name.typename)
  const args = ref.type_arg_list.length
    ? `<${ref.type_arg_list.map(formatTypeArg).join(', ')}>`
    : ''
  return base + args + '[]'.repeat(ref.arr_dim + extraArrayDims)
}

function formatTypeArg(arg: TypeArg): string {
  if (arg === 'Wildcard') return '?'
  if ('Is' in arg) return formatRefType(arg.Is)
  if ('Extends' in arg) return `? extends ${formatRefType(arg.Extends)}`
  return `? super ${formatRefType(arg.Super)}`
}

function formatVoidable(output: VoidableType): string {
  return output === 'Void' ? 'void' : formatRefType(output.RefType)
}

function formatTypeParams(params: TypeParam[]): string {
  if (params.length === 0) return ''
  const rendered = params.map((param) => {
    const name = simpleName(param.name.typename)
    if (param.extends_from.length === 0) return name
    return `${name} extends ${param.extends_from.map((r) => formatRefType(r)).join(' & ')}`
  })
  return `<${rendered.join(', ')}>`
}

/** UML visibility glyph. */
export function accessGlyph(access: AccessModifier): string {
  switch (access) {
    case 'Public':
      return '+'
    case 'Protected':
      return '#'
    case 'Private':
      return '-'
    case 'Default':
      return '~'
  }
}

export type MemberCategory = 'field' | 'method' | 'constructor'

export function memberCategory(member: Member): MemberCategory {
  if ('Property' in member.member_kind) return 'field'
  if ('Constructor' in member.member_kind) return 'method'
  return 'method'
}

/** True for a constructor specifically; constructors sort ahead of methods. */
export function isConstructor(member: Member): boolean {
  return 'Constructor' in member.member_kind
}

/**
 * Renders a member as a UML-style signature, without the visibility glyph:
 *   `loanId: String`
 *   `isOverdue(int): boolean`
 *   `<T> findAll(Class<T>): List<T>`
 */
export function formatMemberSignature(member: Member): string {
  const kind = member.member_kind

  if ('Property' in kind) {
    return `${member.name}: ${formatRefType(kind.Property.reftype, kind.Property.arr_dim)}`
  }

  if ('Constructor' in kind) {
    const { type_param_list, input, throws } = kind.Constructor
    return (
      formatTypeParams(type_param_list) +
      `${member.name}(${input.map((r) => formatRefType(r)).join(', ')})` +
      formatThrows(throws)
    )
  }

  const { type_param_list, input, output, throws } = kind.Method
  return (
    formatTypeParams(type_param_list) +
    `${member.name}(${input.map((r) => formatRefType(r)).join(', ')})` +
    `: ${formatVoidable(output)}` +
    formatThrows(throws)
  )
}

function formatThrows(throws: RefType[]): string {
  return throws.length
    ? ` throws ${throws.map((r) => formatRefType(r)).join(', ')}`
    : ''
}

/** Modifiers worth surfacing next to a member; `final` on a field is noise. */
export function displayModifiers(member: Member): string[] {
  return member.modifiers.modifiers.filter(
    (m) => m !== 'public' && m !== 'private' && m !== 'protected',
  )
}
