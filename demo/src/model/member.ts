import {
  isInProject,
  type AccessModifier,
  type DottedName,
  type FullyQualifiedName,
  type MemberKind,
  type RawMember,
  type RefType,
  type TypeArg,
  type TypeParam,
  type VoidableType,
} from '@/api/types'

export type MemberCategory = 'field' | 'method' | 'constructor'

/**
 * A stable identity for a member, used to join an association edge's inlined
 * `Member` back to the row in its owning type.
 *
 * The backend clones the very same struct into both places, so a canonical
 * serialisation is an exact join key. Names alone are not enough: overloaded
 * methods share a name but are distinct members with distinct edges.
 */
export function memberKey(member: RawMember): string {
  return canonicalize(member)
}

function canonicalize(value: unknown): string {
  if (value === null || typeof value !== 'object') return JSON.stringify(value)
  if (Array.isArray(value)) return `[${value.map(canonicalize).join(',')}]`
  const entries = Object.entries(value as Record<string, unknown>)
  entries.sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))
  return `{${entries.map(([k, v]) => `${JSON.stringify(k)}:${canonicalize(v)}`).join(',')}}`
}

/**
 * Every in-project type this member mentions, anywhere in its signature.
 *
 * The backend's own association edges only look at the *head* of a `RefType`
 * (`build_associative_relationshup_from_node` in `src/abstraction_graph/graph.rs`
 * reads `reftype.name` and stops), so `List<Book>` yields an edge to `List` —
 * which is external, and therefore dropped — and none to `Book`. Annotations and
 * type-parameter bounds are not walked at all.
 *
 * The resolver already resolves type arguments recursively, so the information
 * is present in the payload and simply unused. Walking it here recovers those
 * edges without touching the backend. Measured against the running service, this
 * is a strict superset of the backend's set: `test_target/small` 30 → 43 edges,
 * `test_target/prod` 1574 → 1607, with nothing lost in either.
 */
export function collectMemberTargets(member: RawMember): DottedName[] {
  const found: DottedName[] = []
  const seen = new Set<DottedName>()

  const addName = (fqn: FullyQualifiedName) => {
    // Only project types have a node to point at; `Generic` type-parameter names
    // and external dependencies are rendered as text and nothing more.
    if (!isInProject(fqn.source) || seen.has(fqn.typename)) return
    seen.add(fqn.typename)
    found.push(fqn.typename)
  }

  const addRef = (ref: RefType) => {
    addName(ref.name)
    for (const arg of ref.type_arg_list) {
      if (arg === 'Wildcard') continue
      if ('Is' in arg) addRef(arg.Is)
      else if ('Extends' in arg) addRef(arg.Extends)
      else addRef(arg.Super)
    }
  }

  const addBounds = (params: TypeParam[]) => {
    for (const param of params) param.extends_from.forEach(addRef)
  }

  for (const annotation of member.annotations) addName(annotation.name)

  const kind = member.member_kind
  if ('Property' in kind) {
    addRef(kind.Property.reftype)
  } else if ('Constructor' in kind) {
    addBounds(kind.Constructor.type_param_list)
    kind.Constructor.input.forEach(addRef)
    kind.Constructor.throws.forEach(addRef)
  } else {
    addBounds(kind.Method.type_param_list)
    kind.Method.input.forEach(addRef)
    kind.Method.throws.forEach(addRef)
    if (kind.Method.output !== 'Void') addRef(kind.Method.output.RefType)
  }

  return found
}

export function memberCategory(kind: MemberKind): MemberCategory {
  if ('Property' in kind) return 'field'
  if ('Constructor' in kind) return 'constructor'
  return 'method'
}

/** UML-style visibility glyph. */
export function accessGlyph(access: AccessModifier): string {
  switch (access) {
    case 'Public':
      return '+'
    case 'Private':
      return '-'
    case 'Protected':
      return '#'
    case 'Default':
      return '~'
  }
}

/**
 * How a type reads in a member signature. In-project types drop their package
 * but keep the outer-type path, so `library.model.Book.Builder` reads as
 * `Book.Builder` — which is how it is written in Java source anyway.
 */
export function displayTypeName(fqn: FullyQualifiedName): string {
  if (isInProject(fqn.source)) {
    const pkg = fqn.source.InProjectType.package
    if (pkg && fqn.typename.startsWith(`${pkg}.`)) {
      return fqn.typename.slice(pkg.length + 1)
    }
    return fqn.typename
  }
  return fqn.typename
}

export function formatRefType(ref: RefType, extraArrayDims = 0): string {
  const args = ref.type_arg_list.length
    ? `<${ref.type_arg_list.map(formatTypeArg).join(', ')}>`
    : ''
  const dims = '[]'.repeat(ref.arr_dim + extraArrayDims)
  return `${displayTypeName(ref.name)}${args}${dims}`
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
  if (!params.length) return ''
  const rendered = params.map((p) => {
    const name = displayTypeName(p.name)
    if (!p.extends_from.length) return name
    return `${name} extends ${p.extends_from.map((r) => formatRefType(r)).join(' & ')}`
  })
  return `<${rendered.join(', ')}> `
}

/**
 * The right-hand side of a member row. Note the backend keeps parameter *types*
 * only — parameter names are not retained — so methods read as `save(T): void`.
 */
export function memberSignature(member: RawMember): string {
  const kind = member.member_kind
  if ('Property' in kind) {
    return `${member.name}: ${formatRefType(kind.Property.reftype, kind.Property.arr_dim)}`
  }
  if ('Constructor' in kind) {
    const { type_param_list, input, throws } = kind.Constructor
    return (
      `${formatTypeParams(type_param_list)}${member.name}(${input.map((r) => formatRefType(r)).join(', ')})` +
      formatThrows(throws)
    )
  }
  const { type_param_list, input, output, throws } = kind.Method
  return (
    `${formatTypeParams(type_param_list)}${member.name}(${input.map((r) => formatRefType(r)).join(', ')})` +
    `: ${formatVoidable(output)}${formatThrows(throws)}`
  )
}

function formatThrows(throws: RefType[]): string {
  if (!throws.length) return ''
  return ` throws ${throws.map((r) => formatRefType(r)).join(', ')}`
}
