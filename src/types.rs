use crate::parser::token::OwnedToken;
use core::fmt;
use std::{collections::BTreeSet, path::PathBuf, rc::Rc};

//-----------------------------------------------------------------------
//------------------ ERROR / RESULT TYPES -------------------------------
//-----------------------------------------------------------------------

/// Parse error trait for all parsing stuffs.
pub trait GenericParseResult<T> {
    /// Pushes the current nonterminal's context into the stack.
    fn push_context(self, ctx: (&'static str, usize)) -> Self;
}

/// Error type for our parser
#[derive(Debug, Clone)]
pub enum ParseErrType {
    UnexpectedToken {
        expected: &'static str,
        got: Vec<OwnedToken>,
    },
    UnexpectedEOF,
    LexerError,
    IndexingError,
    UnimplementedError,
    ImportError,
    SemanticError(&'static str),
}

impl ParseErrType {
    pub fn to_stack_parse_err(self, err_index: usize, ctx: (&'static str, usize)) -> ParseErr {
        ParseErr {
            err: self,
            err_index,
            stack: vec![ctx],
        }
    }
}

/// Stacked err uses err and pushes the stack's first index element up onto the stack.
#[derive(Debug, Clone)]
pub struct ParseErr {
    pub err: ParseErrType,
    pub stack: Vec<(&'static str, usize)>,
    pub err_index: usize,
}

/// Result type for stackParseErr
pub type ParseResult<T> = Result<T, ParseErr>;
impl<'a, T> GenericParseResult<T> for ParseResult<T> {
    fn push_context(self, (ctx, index): (&'static str, usize)) -> Self {
        self.map_err(|mut e| {
            e.stack.push((ctx, index));
            e
        })
    }
}

//-----------------------------------------------------------------------
//----------------------- SEMANTIC TYPES --------------------------------
//-----------------------------------------------------------------------

/// A TypeArg has 4 values representing 4 different args:
/// - Is(A) = `A`
/// - Extends(A) = `? extends A`
/// - Super(A) = `? super A`
/// - Wildcard = `?`
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TypeArg {
    Is(RefType),
    Extends(RefType),
    Super(RefType),
    Wildcard,
}

/// A TypeArgList is a list of type args,
/// `<A, B, C>` is translated to `vec![A, B, C]`
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct TypeArgList(pub Vec<TypeArg>);

/// A qualified name is a dotted name, e.g. `java.util.ArrayList`
#[derive(PartialEq, Eq, Clone, Hash)]
pub struct QualifiedName(pub Vec<String>);

impl QualifiedName {
    /// get prefix [0..to)
    /// Note that prefix(i) + suffix(i) = original
    pub fn get_prefix(&self, to: usize) -> Option<Self> {
        Some(Self(self.0.get(0..to)?.to_owned()))
    }

    /// get suffix [to..end]
    /// Note that prefix(i) + suffix(i) = original
    pub fn get_suffix(&self, from: usize) -> Option<Self> {
        Some(Self(self.0.get(from..)?.to_owned()))
    }

    /// Check if self has prefix as a proper prefix.
    ///
    /// Proper prefix is defined as: s is a proper
    /// prefix of S if S = s + k for some nonempty
    /// string k.
    pub fn has_prefix(&self, prefix: &QualifiedName) -> bool {
        self.0.len() > prefix.0.len()
            && prefix
                .0
                .iter()
                .enumerate()
                .all(|(ind, s)| s == &self.0[ind])
    }

    /// generates an postfix (typename in package) seperating from a package name.
    pub fn to_type_no_package(&self, package: &QualifiedName) -> Option<QualifiedName> {
        if !self.has_prefix(package) {
            return None;
        }
        return self.get_suffix(package.0.len());
    }

    /// gets the length (amount of seperated names) from the QN
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl fmt::Debug for QualifiedName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut v = String::new();
        if self.0.len() == 0 {
            f.write_str("0")
        } else {
            v += self.0[0].as_str();
            self.0.iter().skip(1).for_each(|s| {
                v.push('.');
                v.push_str(s.as_str());
            });
            f.write_str(&v)
        }
    }
}

/// A struct to represent type usages with generic,
/// e.g. `java.util.Hashtable<Integer, ? extends com.util.MyClass>`
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RefType {
    pub name: QualifiedName,
    pub type_arg_list: TypeArgList,
    pub arr_dim: u8,
}

/// An annotation is a string slice of one annotation for some type/property/method
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Annotation {
    pub name: QualifiedName,
    pub s: String,
}

/// A voidable type is an output for a function.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum VoidableType {
    Void,
    RefType(RefType),
}

/// A list of parameters for generic types
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct TypeParamList(pub Vec<TypeParam>);

/// A type param is an input type (class `BinaryTree<K extends Comparable<K>, V>`,
/// then `<K extends Comparable<K>>` and `<V>` are type params)
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct TypeParam {
    pub name: String,
    pub extends_from: Vec<RefType>,
}

/// A member can be a method or a property.
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
    pub annotations: Vec<Rc<Annotation>>,
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
    pub annotation: Vec<Rc<Annotation>>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ImportObject {
    pub name: QualifiedName,
    pub is_static: bool,
    pub is_wildcard: bool,
}

#[derive(Debug, PartialEq, Clone, Copy, Eq, PartialOrd, Ord)]
pub enum AccessModifier {
    Private,
    Default,
    Protected,
    Public,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Modifiers {
    pub modifiers: BTreeSet<String>,
    pub access_modifier: AccessModifier,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct JavaFile {
    /// None means this is in the default package
    pub package_name: QualifiedName,
    /// imported objects, could be com.etc.*
    pub imported_objects: Vec<ImportObject>,
    /// type declarations in the current file
    pub type_decls: Vec<Type>,
    /// the file that reads this.
    pub file: Rc<PathBuf>,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_access_modifier_ordering() {
        use AccessModifier::*;
        assert!(Public > Protected);
        assert!(Protected > Default);
        assert!(Default > Private);
    }
}
