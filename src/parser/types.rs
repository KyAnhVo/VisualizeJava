use crate::parser::token::Token;

/// Error type for our parser
pub enum ParseErr<'a> {
    UnexpectedToken {
        expected: String,
        got: Vec<Token<'a>>,
    },
    UnexpectedEOF,
    LexerError,
    IndexingError,
    UnimplementedError,
}
pub type Result<'a, T> = std::result::Result<T, ParseErr<'a>>;

/// A TypeArg has 4 values representing 4 different args:
/// - Is(A) = `A`
/// - Extends(A) = `? extends A`
/// - Super(A) = `? super A`
/// - Wildcard = `?`
#[derive(Debug, PartialEq)]
pub enum TypeArg<'a> {
    Is(RefType<'a>),
    Extends(RefType<'a>),
    Super(RefType<'a>),
    Wildcard,
}

/// A TypeArgList is a list of type args,
/// `<A, B, C>` is translated to `vec![A, B, C]`
#[derive(Debug, PartialEq)]
pub struct TypeArgList<'a>(pub Vec<TypeArg<'a>>);

/// A qualified name is a dotted name, e.g. `java.util.ArrayList`
#[derive(Debug, PartialEq)]
pub struct QualifiedName<'a>(pub Vec<&'a str>);

/// A struct to represent type usages with generic,
/// e.g. `java.util.Hashtable<Integer, ? extends com.util.MyClass>`
#[derive(Debug, PartialEq)]
pub struct RefType<'a> {
    pub name: QualifiedName<'a>,
    pub type_arg_lst: TypeArgList<'a>,
    pub arr_dim: usize,
}

/// A voidable type is an output for a function.
#[derive(Debug, PartialEq)]
pub enum VoidableType<'a> {
    Void,
    RefType(RefType<'a>),
}

/// A member can be a method or a property.
#[derive(Debug, PartialEq)]
pub enum MemberKind<'a> {
    Property {
        reftype: RefType<'a>,
    },
    Method {
        input: RefType<'a>,
        output: VoidableType<'a>,
    },
}

#[derive(Debug, PartialEq)]
pub struct Member<'a> {
    pub member_kind: MemberKind<'a>,
    pub annotations: Vec<&'a str>,
}

/// A typekind is an enum of different kinds of type
#[derive(Debug, PartialEq)]
pub enum TypeKind<'a> {
    Class {
        inherits_from: RefType<'a>,
        implement_interfaces: Vec<RefType<'a>>,
    },
    Enum {
        implement_interfaces: Vec<RefType<'a>>,
    },
    Interface {
        extend_interfaces: Vec<RefType<'a>>,
    },
    Annotation {},
}

/// A type can be a class/enum/interface/annotation.
#[derive(Debug, PartialEq)]
pub struct Type<'a> {
    pub name: &'a str,
    pub type_kind: TypeKind<'a>,
    pub members: Vec<Member<'a>>,
    pub subtypes: Vec<Type<'a>>,
    pub annotation: Vec<Type<'a>>,
}

pub struct JavaFile<'a> {
    /// None means this is in the default package
    pub package_name: Option<QualifiedName<'a>>,
    /// imported objects, could be com.etc.*
    pub imported_objects: Vec<QualifiedName<'a>>,
    /// type declarations in the current file
    pub type_decls: Vec<Type<'a>>,
}
