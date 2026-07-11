use std::path::PathBuf;
use std::{collections::HashMap, path::Path, rc::Rc};
use std::{fs, io};

use crate::types::{
    AccessModifier, ImportObject, JavaFile, Member, Modifiers, ParseErr, QualifiedName, Type,
    TypeKind,
};

#[derive(Debug)]
pub enum ProjErr {
    IoErr(io::Error),
    ParseErr(ParseErr),
}

#[derive(Debug, PartialEq)]
pub struct FlattenType {
    pub name: QualifiedName,
    pub modifiers: Modifiers,
    pub members: Rc<[Member]>,
    pub type_kind: TypeKind,
    pub import_objs: Rc<[ImportObject]>,
}

impl FlattenType {
    pub fn from_file(file: &JavaFile) -> Vec<Self> {
        let import_objs: Rc<[ImportObject]> = Rc::from(file.imported_objects.clone());
        let mut res: Vec<Self> = vec![];
        for typeclass in file.type_decls.iter() {
            res.append(&mut Self::from_type(&typeclass, import_objs.clone()));
        }
        res
    }

    /// Flattens a type into a vector of types
    pub fn from_type(typeclass: &Type, import_objs: Rc<[ImportObject]>) -> Vec<Self> {
        let import_objs: Rc<[ImportObject]> = Rc::from(import_objs);
        Self::recursive_from_type(typeclass, import_objs.clone(), AccessModifier::Public)
    }

    fn recursive_from_type(
        typeclass: &Type,
        import_objs: Rc<[ImportObject]>,
        min_visibility: AccessModifier,
    ) -> Vec<Self> {
        let name = typeclass.name.clone();
        let modifiers = Modifiers {
            modifiers: typeclass.modifiers.modifiers.clone(),
            access_modifier: typeclass.modifiers.access_modifier.min(min_visibility),
        };
        let members: Rc<[Member]> = Rc::from(typeclass.body.members.clone());
        let type_kind = typeclass.type_kind.clone();

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
            .iter()
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
pub struct Project(HashMap<QualifiedName, Vec<FlattenType>>);

impl Project {
    fn get_java_files(root_dir: &Path) -> io::Result<Vec<PathBuf>> {
        if !root_dir.is_dir() {
            if root_dir.extension().is_some_and(|x| x.eq("java")) {
                return Ok(vec![root_dir.to_path_buf()]);
            } else {
                return Ok(vec![]);
            }
        }

        let mut v: Vec<PathBuf> = vec![];
        let entries = fs::read_dir(root_dir)?;
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
        let file = Parser::parse(
            "
            package com.example;
            public class BinaryTree {
                public static class Node {
                    public String id;
                    public Node left = Null, right = Null;
                }
                public Node root;
            }
            class Graph {
                public static class Node {
                    public String id;
                    public Vector<Node> to_nodes;
                }
                public HashMap<String, Node> nodes;
            }
            ",
        )
        .unwrap();
        let mut res: Vec<FlattenType> = vec![];
        let import_objs: Rc<[ImportObject]> = Rc::from(file.imported_objects);
        for typeclass in file.type_decls.iter() {
            res.append(&mut FlattenType::from_type(typeclass, import_objs.clone()));
        }
        assert_eq!(
            res[0].name,
            QualifiedName(vec![
                "com".to_owned(),
                "example".to_owned(),
                "BinaryTree".to_owned()
            ])
        );
        assert_eq!(
            res[1].name,
            QualifiedName(vec![
                "com".to_owned(),
                "example".to_owned(),
                "BinaryTree".to_owned(),
                "Node".to_owned()
            ])
        );
        assert_eq!(
            res[2].name,
            QualifiedName(vec![
                "com".to_owned(),
                "example".to_owned(),
                "Graph".to_owned(),
            ])
        );
        assert_eq!(
            res[3].name,
            QualifiedName(vec![
                "com".to_owned(),
                "example".to_owned(),
                "Graph".to_owned(),
                "Node".to_owned(),
            ])
        );
        assert_eq!(res[0].members[0].name, "root");
        assert_eq!(res[1].members[0].name, "id");
        assert_eq!(res[1].members[1].name, "left");
        assert_eq!(res[1].members[2].name, "right");
    }
}
