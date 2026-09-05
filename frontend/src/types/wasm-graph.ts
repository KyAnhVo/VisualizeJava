/**
 * TypeScript mirror of the values `ProjectBuilder.build_graph()` produces.
 *
 * The wasm side serializes with `serde_wasm_bindgen`, which means:
 *   - Rust structs become plain objects with snake_case keys.
 *   - Rust `HashMap` becomes a JS `Map`, *not* an object.
 *   - Externally-tagged enums become either a bare string (unit variants) or a
 *     single-key object `{ VariantName: payload }`.
 *   - `QualifiedName` has a hand-written `Serialize` that joins on `.`, so it
 *     arrives as a dotted string rather than an array.
 *
 * Keys of the top-level map are the value of `FullyQualifiedName::into_fqn()`,
 * i.e. the dotted fully-qualified type name (`library.model.Loan`). Nested
 * types use their enclosing type as a segment (`library.model.Book.Builder`).
 */

/** Dotted name, e.g. `java.util.List` or `library.model.Book.Builder`. */
export type QualifiedName = string

export type PrimitiveType =
  | 'Int'
  | 'Boolean'
  | 'Char'
  | 'Byte'
  | 'Short'
  | 'Long'
  | 'Float'
  | 'Double'

export type TypeSource =
  | { InProjectType: { package: QualifiedName } }
  | { PrimitiveType: PrimitiveType }
  | 'ExternalDependencyType'
  | 'Generic'
  | 'Ambiguous'

export interface FullyQualifiedName {
  source: TypeSource
  typename: QualifiedName
}

export interface RefType {
  name: FullyQualifiedName
  type_arg_list: TypeArg[]
  arr_dim: number
}

export type TypeArg =
  | { Is: RefType }
  | { Extends: RefType }
  | { Super: RefType }
  | 'Wildcard'

export type VoidableType = 'Void' | { RefType: RefType }

export interface TypeParam {
  name: FullyQualifiedName
  extends_from: RefType[]
}

export interface Annotation {
  name: FullyQualifiedName
  s: string
}

export type AccessModifier = 'Private' | 'Default' | 'Protected' | 'Public'

export interface Modifiers {
  /** Serialized from a `BTreeSet<String>`, so alphabetically sorted. */
  modifiers: string[]
  access_modifier: AccessModifier
}

export type MemberKind =
  | { Property: { reftype: RefType; arr_dim: number } }
  | {
      Method: {
        type_param_list: TypeParam[]
        input: RefType[]
        output: VoidableType
        throws: RefType[]
      }
    }
  | {
      Constructor: {
        type_param_list: TypeParam[]
        input: RefType[]
        throws: RefType[]
      }
    }

export interface Member {
  name: string
  member_kind: MemberKind
  annotations: Annotation[]
  modifiers: Modifiers
}

/** `Enum` carries its constant names. */
export type TypeVariant =
  | 'Class'
  | 'Interface'
  | 'Annotation'
  | { Enum: string[] }

/**
 * `variant` is `Association` when the edge came from a field, parameter, return
 * type, throws clause or type parameter; the payload is the member responsible.
 */
export type EdgeVariant = 'Extends' | 'Implements' | { Association: Member }

export interface WasmEdge {
  /** The *other* end: the target for `out_edges`, the source for `in_edges`. */
  typename: FullyQualifiedName
  variant: EdgeVariant
}

export interface WasmNode {
  name: FullyQualifiedName
  type_variant: TypeVariant
  members: Member[]
  out_edges: WasmEdge[]
  in_edges: WasmEdge[]
}

/**
 * The graph is symmetric: every edge recorded in one node's `out_edges` also
 * appears in the target's `in_edges`. Traversing `out_edges` alone therefore
 * visits each edge exactly once.
 *
 * Edges whose endpoints are not both declared inside the uploaded project are
 * dropped by the wasm side, so every `typename` here resolves to a key of this
 * map.
 */
export type WasmGraph = Map<QualifiedName, WasmNode>
