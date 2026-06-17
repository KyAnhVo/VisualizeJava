use crate::parser::token::Token;

/// Error type for our parser
#[derive(Debug)]
pub enum ParseErr<'a> {
    UnexpectedToken {
        expected: &'static str,
        got: Vec<Token<'a>>,
    },
    UnexpectedEOF,
    LexerError,
    IndexingError,
    UnimplementedError,
    ImportError,
    MultiplePublicTypesError,
}

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
pub type ParseResult<'a, T> = Result<T, ParseErr<'a>>;

/// A TypeArgList is a list of type args,
/// `<A, B, C>` is translated to `vec![A, B, C]`
#[derive(Debug, PartialEq)]
pub struct TypeArgList<'a>(pub Vec<TypeArg<'a>>);

/// A qualified name is a dotted name, e.g. `java.util.ArrayList`
#[derive(Debug, PartialEq, Clone)]
pub struct QualifiedName<'a>(pub Vec<&'a str>);

/// An annotation is a string slice of one annotation for some type/property/method
#[derive(Debug, PartialEq, Clone)]
pub struct Annotation<'a>(pub &'a str);

/// A struct to represent type usages with generic,
/// e.g. `java.util.Hashtable<Integer, ? extends com.util.MyClass>`
#[derive(Debug, PartialEq)]
pub struct RefType<'a> {
    pub name: QualifiedName<'a>,
    pub type_arg_list: TypeArgList<'a>,
    pub arr_dim: u8,
}

/// A voidable type is an output for a function.
#[derive(Debug, PartialEq)]
pub enum VoidableType<'a> {
    Void,
    RefType(RefType<'a>),
}

/// A list of parameters for generic types
#[derive(Debug, PartialEq)]
pub struct TypeParamList<'a>(pub Vec<TypeParam<'a>>);

/// A type param is an input type (class `BinaryTree<K Comparable<K>, V>`,
/// then `<K extends Comparable<K>>` and `<V>` are type params)
#[derive(Debug, PartialEq)]
pub struct TypeParam<'a> {
    pub name: &'a str,
    pub extends_from: Vec<RefType<'a>>,
}

/// A member can be a method or a property.
#[derive(Debug, PartialEq)]
pub enum MemberKind<'a> {
    Property {
        reftype: RefType<'a>,
    },
    Method {
        type_param_list: TypeParamList<'a>,
        input: Vec<RefType<'a>>,
        output: VoidableType<'a>,
        throws: Vec<RefType<'a>>,
    },
}

#[derive(Debug, PartialEq)]
pub struct Member<'a> {
    pub name: &'a str,
    pub member_kind: MemberKind<'a>,
    pub annotations: Vec<Annotation<'a>>,
    pub modifiers: Modifiers<'a>,
}

/// A typekind is an enum of different kinds of type
#[derive(Debug, PartialEq)]
pub enum TypeKind<'a> {
    Class {
        inherits_from: Option<RefType<'a>>,
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

/// A type's body contains its members (not subtypes) and its subtypes.
#[derive(Debug, PartialEq)]
pub struct TypeBody<'a> {
    pub members: Vec<Member<'a>>,
    pub subtypes: Vec<Type<'a>>,
}

/// A type can be a class/enum/interface/annotation.
#[derive(Debug, PartialEq)]
pub struct Type<'a> {
    pub name: QualifiedName<'a>,
    pub modifiers: Modifiers<'a>,
    pub type_kind: TypeKind<'a>,
    pub body: TypeBody<'a>,
    pub annotation: Vec<Annotation<'a>>,
}
pub struct JavaFile<'a> {
    /// None means this is in the default package
    pub package_name: Option<QualifiedName<'a>>,
    /// imported objects, could be com.etc.*
    pub imported_objects: Vec<ImportObject<'a>>,
    /// type declarations in the current file
    pub type_decls: Vec<Type<'a>>,
}

pub struct ImportObject<'a> {
    pub name: QualifiedName<'a>,
    pub is_static: bool,
    pub is_wildcard: bool,
}

#[derive(Debug, PartialEq)]
pub enum AccessModifier {
    Public,
    Private,
    Protected,
    Default,
}

#[derive(Debug, PartialEq)]
pub struct Modifiers<'a> {
    pub modifiers: Vec<&'a str>,
    pub access_modifier: AccessModifier,
}
