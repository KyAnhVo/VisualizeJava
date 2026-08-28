/**
 * TypeScript mirror of the JSON the Rust backend emits from `POST /graph`.
 *
 * These types are a hand transcription of the `#[derive(Serialize)]` structs in
 * `src/abstraction_graph/graph_types.rs` and `src/resolved_types.rs`. Rust enums
 * use serde's default *externally tagged* representation: a unit variant becomes
 * the bare string `"Extends"`, a data-carrying variant becomes the single-key
 * object `{ "Association": … }`.
 *
 * `QualifiedName` has a custom `Serialize` impl (`src/types.rs`) that joins the
 * segments with `.`, so every name arrives as a plain dotted string.
 */

/** e.g. `"library.model.Book"`, or `"String"` for an unresolved external type. */
export type DottedName = string

export type PrimitiveType =
  | 'Int'
  | 'Boolean'
  | 'Char'
  | 'Byte'
  | 'Short'
  | 'Long'
  | 'Float'
  | 'Double'

/**
 * Where a referenced type came from. Only `InProjectType` types get their own
 * node in the graph; everything else is a leaf we render as plain text.
 */
export type TypeSource =
  | { InProjectType: { package: DottedName } }
  | { PrimitiveType: PrimitiveType }
  | 'ExternalDependencyType'
  | 'Generic'
  | 'Ambiguous'

export interface FullyQualifiedName {
  source: TypeSource
  /** For `InProjectType` this includes the package *and* any outer types. */
  typename: DottedName
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

export interface RawAnnotation {
  name: FullyQualifiedName
  /** The annotation as written in source, e.g. `@Field(label = "ISBN")`. */
  s: string
}

export type AccessModifier = 'Private' | 'Default' | 'Protected' | 'Public'

export interface Modifiers {
  /** Sorted (`BTreeSet` on the Rust side) — includes the access modifier too. */
  modifiers: string[]
  access_modifier: AccessModifier
}

/**
 * Note that `input` is a list of parameter *types* only — the backend does not
 * retain parameter names, so signatures render as `save(T): void`.
 */
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

export interface RawMember {
  name: string
  member_kind: MemberKind
  annotations: RawAnnotation[]
  modifiers: Modifiers
}

export type TypeVariant =
  | 'Class'
  | 'Interface'
  | 'Annotation'
  | { Enum: string[] }

/**
 * `Extends` is emitted only for class-extends-class. `Implements` covers both
 * `class implements I` *and* `interface A extends B` — see
 * `build_inheritance_relationship_from_node` in `src/abstraction_graph/graph.rs`.
 */
export type EdgeVariant = 'Extends' | 'Implements' | { Association: RawMember }

export interface RawEdge {
  /** The *other* end of the edge — the target for `out_edges`, source for `in_edges`. */
  typename: FullyQualifiedName
  variant: EdgeVariant
}

export interface RawNode {
  name: FullyQualifiedName
  type_variant: TypeVariant
  members: RawMember[]
  /** `out_edges` and `in_edges` are mirror images; build from one side only. */
  out_edges: RawEdge[]
  in_edges: RawEdge[]
}

/** The whole payload: a flat map from fully-qualified name to node. */
export type RawGraph = Record<DottedName, RawNode>

// --- narrowing helpers -------------------------------------------------------

export function isInProject(
  source: TypeSource,
): source is { InProjectType: { package: DottedName } } {
  return typeof source === 'object' && 'InProjectType' in source
}

export function isAssociation(
  variant: EdgeVariant,
): variant is { Association: RawMember } {
  return typeof variant === 'object' && 'Association' in variant
}

export function isEnum(variant: TypeVariant): variant is { Enum: string[] } {
  return typeof variant === 'object' && 'Enum' in variant
}
