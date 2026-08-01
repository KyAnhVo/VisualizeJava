use std::collections::HashMap;

use crate::resolved_types::{FullyQualifiedName, TypeKind};
use crate::types::QualifiedName;

#[derive(Debug, PartialEq, Eq)]
pub enum TypeVariant {
    Class,
    Enum,
    Interface,
    Annotation,
}

impl TypeVariant {
    pub fn from_typekind(typekind: &TypeKind) -> Self {
        match typekind {
            TypeKind::Class { .. } => Self::Class,
            TypeKind::Enum { .. } => Self::Enum,
            TypeKind::Interface { .. } => TypeVariant::Enum,
            TypeKind::Annotation { .. } => TypeVariant::Annotation,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum EdgeVariant {
    Extends,
    Implements,
    MemberType,
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
            (Enum, _) => panic!("enum is final"),
        }
    }
}

#[derive(Debug, Hash)]
pub struct Name {
    pub pkg_name: QualifiedName,
    pub name: QualifiedName,
}

#[derive(Debug)]
pub struct Property {
    pub typename: FullyQualifiedName,
    pub name: String,
}

#[derive(Debug)]
pub struct Edge {
    pub typename: FullyQualifiedName,
    pub variant: EdgeVariant,
}

/// stores metadata and graph stuffs for nodes
#[derive(Debug)]
pub struct Node {
    pub name: FullyQualifiedName,
    pub type_variant: TypeVariant,
    pub properties: Vec<Property>,

    pub out_edges: Vec<Edge>,
    pub in_edges: Vec<Edge>,
}
