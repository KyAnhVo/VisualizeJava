use std::ffi::OsStr;
use std::path::PathBuf;
use std::{collections::HashMap, path::Path, rc::Rc};
use std::{fs, io};

use crate::parser::parser::Parser;
use crate::types::{
    AccessModifier, ImportObject, JavaFile, Member, Modifiers, QualifiedName, Type, TypeBody,
    TypeKind,
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
    pub fn from_file(file: JavaFile<'a>) -> Vec<Self> {
        let import_objs: Rc<[ImportObject<'a>]> = Rc::from(file.imported_objects);
        let mut res: Vec<Self> = vec![];
        for typeclass in file.type_decls {
            res.append(&mut Self::from_type(typeclass, import_objs.clone()));
        }
        res
    }

    /// Flattens a type into a vector of types
    pub fn from_type(typeclass: Type<'a>, import_objs: Rc<[ImportObject<'a>]>) -> Vec<Self> {
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
            modifiers: modifiers.clone(),
            members,
            type_kind,
            import_objs: import_objs.clone(),
        };

        let mut res = vec![flatten_type];
        let mut inner_types: Vec<Self> = typeclass
            .body
            .subtypes
            .into_iter()
            .map(|typeclass| {
                Self::recursive_from_type(typeclass, import_objs.clone(), modifiers.access_modifier)
            })
            .flatten()
            .collect();
        res.append(&mut inner_types);
        res
    }
}

/// A Project is an wrapper for a hashmap
/// ```
/// f: QualifiedName -> Rc<[FlattenType]>
/// ```
/// where semantically it maps a package name to all of its declared types.
pub struct Project<'a>(HashMap<QualifiedName<'a>, Vec<FlattenType<'a>>>);

impl<'a> Project<'a> {
    pub fn new(dir: &str) -> io::Result<Self> {
        let java_files = Self::get_java_files(PathBuf::from(dir).as_path())?;
        let hashmap: HashMap<QualifiedName, Vec<FlattenType>> = HashMap::new();

        for file in java_files {
            let content = fs::read_to_string(file)?;
        }

        Ok(Self(hashmap))
    }

    fn get_java_files(dir: &Path) -> io::Result<Vec<PathBuf>> {
        if !dir.is_dir() {
            if dir.extension().is_some_and(|x| x.eq("java")) {
                return Ok(vec![dir.to_path_buf()]);
            } else {
                return Ok(vec![]);
            }
        }

        let mut v: Vec<PathBuf> = vec![];
        let entries = fs::read_dir(dir)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            v.append(&mut Self::get_java_files(&path)?);
        }

        Ok(v)
    }
}

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
                    public Node left = Null, right = Null;
                }
                public Node root;
            }
            ",
        );

        let file = parser.unwrap().parse().unwrap();
        let mut res: Vec<FlattenType> = vec![];
        let import_objs: Rc<[ImportObject]> = Rc::from(file.imported_objects);
        for typeclass in file.type_decls {
            res.append(&mut FlattenType::from_type(typeclass, import_objs.clone()));
        }
        assert_eq!(
            res[0].name,
            QualifiedName(vec!["com", "example", "BinaryTree"])
        );
        assert_eq!(
            res[1].name,
            QualifiedName(vec!["com", "example", "BinaryTree", "Node"])
        );
        assert_eq!(res[0].members[0].name, "root");
        assert_eq!(res[1].members[0].name, "id");
        assert_eq!(res[1].members[1].name, "left");
        assert_eq!(res[1].members[2].name, "right");
    }
}
