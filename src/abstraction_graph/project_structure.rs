use std::{collections::HashMap, rc::Rc};

use crate::types::{
    AccessModifier, ImportObject, Member, Modifiers, QualifiedName, Type, TypeBody, TypeKind,
};

#[derive(Debug, PartialEq)]
pub struct FlattenType<'a> {
    pub name: QualifiedName<'a>,
    pub modifiers: Modifiers<'a>,
    pub members: Rc<[Member<'a>]>,
    pub type_kind: TypeKind<'a>,
    pub import_objs: Rc<[ImportObject<'a>]>,
}

impl<'a> FlattenType<'a> {
    pub fn from_type(typeclass: Type<'a>, import_objs: Vec<ImportObject<'a>>) -> Vec<Self> {
        let import_objs: Rc<[ImportObject<'a>]> = Rc::from(import_objs);
        Self::recursive_from_type(typeclass, import_objs.clone(), AccessModifier::Public)
    }

    fn recursive_from_type(
        typeclass: Type<'a>,
        import_objs: Rc<[ImportObject<'a>]>,
        min_visibility: AccessModifier,
    ) -> Vec<Self> {
        let name = typeclass.name;
        let modifiers = Modifiers {
            modifiers: typeclass.modifiers.modifiers,
            access_modifier: typeclass.modifiers.access_modifier.min(min_visibility),
        };
        let members: Rc<[Member<'a>]> = Rc::from(typeclass.body.members);
        let type_kind = typeclass.type_kind;

        let flatten_type = Self {
            name,
            modifiers,
            members,
            type_kind,
            import_objs,
        };

        let mut inner_types: Vec<Self> = typeclass
            .body
            .subtypes
            .into_iter()
            .map(|typeclass| {
                Self::recursive_from_type(
                    typeclass,
                    flatten_type.import_objs.clone(),
                    flatten_type.modifiers.access_modifier,
                )
            })
            .flatten()
            .collect();
        inner_types.push(flatten_type);
        vec![]
    }
}

/// A Project is an wrapper for a hashmap
/// ```
/// f: QualifiedName -> Rc<[FlattenType]>
/// ```
/// where semantically it maps a package name to all of its declared types.
pub struct Project<'a>(HashMap<QualifiedName<'a>, Rc<[FlattenType<'a>]>>);

impl<'a> Project<'a> {}

#[cfg(test)]
mod test {
    use crate::parser::parser::Parser;

    use super::*;

    #[test]
    fn test_type_flattening() {
        let parser = Parser::new(
            "
            package com.example;
            public class BinaryTree {
                public static class Node {
                    public String id;
                    public Node left;
                    public Node right;
                }
                public Node root;
            }2q all            ",
        );

        let mut file = parser.unwrap().parse().unwrap();
        for typeclass in file.type_decls {}
    }
}
