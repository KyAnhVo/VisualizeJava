use std::rc::Rc;

use crate::types::{AccessModifier, QualifiedName};

/// A fully qualified name denotes a package and a type.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct FullyQualifiedName {
    /// None denotes no source (from the current project, maybe
    /// from external dependency or from java.*)
    pub package: Option<QualifiedName>,
    pub typename: QualifiedName,
}

impl FullyQualifiedName {
    pub fn into_fqn(&self) -> QualifiedName {
        let mut v = if let Some(p) = self.package.clone() {
            p.0
        } else {
            vec![]
        };
        v.extend_from_slice(self.typename.0.as_ref());
        return QualifiedName(v);
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RefType {
    pub name: FullyQualifiedName,
    pub type_arg_list: TypeArgList,
    pub arr_dim: u8,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum VoidableType {
    Void,
    RefType(RefType),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TypeArg {
    Is(RefType),
    Extends(RefType),
    Super(RefType),
    Wildcard,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct TypeArgList(pub Vec<TypeArg>);

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct TypeParam {
    pub name: FullyQualifiedName,
    pub extends_from: Vec<RefType>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct TypeParamList(pub Vec<TypeParam>);

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Annotation {
    pub name: QualifiedName,
    pub s: String,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Modifiers {
    pub modifiers: Vec<String>,
    pub access_modifier: AccessModifier,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum MemberKind {
    Property {
        reftype: RefType,
        arr_dim: u8,
    },
    Method {
        type_param_list: TypeParamList,
        input: Vec<RefType>,
        output: VoidableType,
        throws: Vec<RefType>,
    },
    Constructor {
        type_param_list: TypeParamList,
        input: Vec<RefType>,
        throws: Vec<RefType>,
    },
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Member {
    pub name: String,
    pub member_kind: MemberKind,
    pub annotations: Vec<Annotation>,
    pub modifiers: Modifiers,
}

/// A typekind is an enum of different kinds of type
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TypeKind {
    Class {
        inherit_class: Option<RefType>,
        implement_interfaces: Vec<RefType>,
    },
    Enum {
        implement_interfaces: Vec<RefType>,
        enum_vals: Vec<String>,
    },
    Interface {
        extend_interfaces: Vec<RefType>,
    },
    Annotation {
        annotation_properties: Vec<(String, RefType)>,
    },
}

/// A type's body contains its members (not subtypes) and its subtypes.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct TypeBody {
    pub members: Vec<Member>,
    pub subtypes: Vec<Type>,
}

/// A type can be a class/enum/interface/annotation.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Type {
    pub name: QualifiedName,
    pub modifiers: Modifiers,
    pub type_kind: TypeKind,
    pub body: TypeBody,
    pub annotation: Vec<Annotation>,
}

/// this is the AST when fully resolved.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct FileTypeTree(pub Vec<Rc<Type>>);

pub struct PackageTypeTree(pub Vec<Rc<Type>>);
