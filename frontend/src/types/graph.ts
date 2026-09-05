import type { Member } from './wasm-graph'

/** The three relationships the abstraction graph records. */
export type RelationKind = 'extends' | 'implements' | 'association'

export type TypeVariantKind = 'class' | 'interface' | 'enum' | 'annotation'

export interface ProjectType {
  /** Fully-qualified dotted name; unique, and used as the React Flow node id. */
  id: string
  /** Last segment of the FQN, e.g. `Book` (or `Book.Builder` for a nested type). */
  simpleName: string
  /** Declaring package, or `''` for the default package. */
  packageName: string
  variant: TypeVariantKind
  /** Constant names, present only when `variant === 'enum'`. */
  enumValues: string[]
  members: Member[]
}

/**
 * One relationship between two project types. Parallel association edges (a
 * class referencing another through several members) are collapsed into a
 * single edge, with every contributing member name kept in `via`.
 */
export interface ProjectRelation {
  id: string
  source: string
  target: string
  kind: RelationKind
  /** Member names that produced an association edge; empty for inheritance. */
  via: string[]
}

export interface ProjectGraph {
  types: ProjectType[]
  relations: ProjectRelation[]
  /** Every distinct package present, sorted, for colouring and the legend. */
  packages: string[]
}
