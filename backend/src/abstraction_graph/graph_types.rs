use serde::Serialize;

use crate::resolved_types::{FullyQualifiedName, Member, TypeKind};
use crate::types::QualifiedName;
use std::rc::Rc;

#[derive(Debug, PartialEq, Eq, Serialize)]
pub enum TypeVariant {
    Class,
    Enum(Vec<String>),
    Interface,
    Annotation,
}

impl TypeVariant {
    pub fn from_typekind(typekind: &TypeKind) -> Self {
        match typekind {
            TypeKind::Class { .. } => Self::Class,
            TypeKind::Enum { enum_vals, .. } => Self::Enum(enum_vals.clone()),
            TypeKind::Interface { .. } => TypeVariant::Interface,
            TypeKind::Annotation { .. } => TypeVariant::Annotation,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize)]
pub enum EdgeVariant {
    Extends,
    Implements,
    Association(Rc<Member>),
}

impl EdgeVariant {
    /// Genrate a graph based on inheritance edge
    /// from and to must exhibit inheritance relationship:
    /// `public class To extends From1 implements From2, From3 {...}`
    pub fn from_inheritance(from: &Node, to: &Node) -> Self {
        use EdgeVariant::*;
        use TypeVariant::*;
        match (&from.type_variant, &to.type_variant) {
            (Class, Class) => Extends,
            (Class, _) => panic!("only class can extend class"),
            (Interface, _) => Implements,
            (Annotation, _) => panic!("annotation cannot extend anything"),
            (Enum(_), _) => panic!("enum is final"),
        }
    }
}

#[derive(Debug, Hash, Serialize)]
pub struct Name {
    pub pkg_name: QualifiedName,
    pub name: QualifiedName,
}

#[derive(Debug, Serialize)]
pub struct Edge {
    pub typename: FullyQualifiedName,
    pub variant: EdgeVariant,
}

/// stores metadata and graph stuffs for nodes
#[derive(Debug, Serialize)]
pub struct Node {
    pub name: FullyQualifiedName,
    pub type_variant: TypeVariant,

    pub members: Rc<[Rc<Member>]>,
    pub out_edges: Vec<Edge>,
    pub in_edges: Vec<Edge>,
}
