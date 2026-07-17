use std::rc::Rc;

use crate::types::{Modifiers, QualifiedName};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum PrimitiveType {
    Int,
    Boolean,
    Char,
    Byte,
    Short,
    Long,
    Float,
    Double,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TypeSource {
    InProjectType {
        package: QualifiedName,
    },
    PrimitiveType(PrimitiveType),
    /// Any type outside the project (java.*, javafx, third-party deps like Spring, etc).
    /// We don't resolve or distinguish these further, so no origin is tracked.
    ExternalDependencyType,
    Generic,
}

/// A fully qualified name denotes a package and a type.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct FullyQualifiedName {
    pub source: TypeSource,
    pub typename: QualifiedName,
}

impl FullyQualifiedName {
    pub fn into_fqn(&self) -> QualifiedName {
        match self.source {
            TypeSource::InProjectType { ref package } => {
                let mut name = package.clone();
                name.0.append(&mut self.typename.0.clone());
                name
            }
            TypeSource::PrimitiveType(ref prim) => match prim {
                PrimitiveType::Int => QualifiedName(vec!["int".to_owned()]),
                PrimitiveType::Boolean => QualifiedName(vec!["boolean".to_owned()]),
                PrimitiveType::Char => QualifiedName(vec!["char".to_owned()]),
                PrimitiveType::Byte => QualifiedName(vec!["byte".to_owned()]),
                PrimitiveType::Short => QualifiedName(vec!["short".to_owned()]),
                PrimitiveType::Long => QualifiedName(vec!["long".to_owned()]),
                PrimitiveType::Float => QualifiedName(vec!["float".to_owned()]),
                PrimitiveType::Double => QualifiedName(vec!["double".to_owned()]),
            },
            TypeSource::ExternalDependencyType => self.typename.clone(),
            TypeSource::Generic => self.typename.clone(),
        }
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
