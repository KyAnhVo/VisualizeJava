use crate::name_resolution::err::ReadProjectErr;
use crate::name_resolution::file_util::{Stack, get_java_files_recursive};
use crate::parser;
use crate::types::{
    AccessModifier, ImportObject, JavaFile, Member, Modifiers, QualifiedName, Type, TypeKind,
};
use std::collections::HashMap;
use std::collections::hash_map::Keys;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

#[derive(Debug)]
pub struct FlattenProject(HashMap<QualifiedName, FlattenPackage>);

impl FlattenProject {
    pub fn new(root_dir: PathBuf) -> Result<Self, ReadProjectErr> {
        let files = get_java_files_recursive(&root_dir.to_path_buf(), &root_dir.to_path_buf())?;
        let mut proj: Self = Self(HashMap::new());
        for (path, _) in files.into_iter() {
            let ast =
                parser::parser::Parser::parse(fs::read_to_string(path.clone()).unwrap().as_str())
                    .map_err(|e| ReadProjectErr::ParseErr(e, path))?;
            let file = FlattenFile::from_file(&ast)?;
            match proj.0.get_mut(&ast.package_name) {
                None => {
                    proj.0
                        .insert(ast.package_name.clone(), FlattenPackage(vec![file]));
                }
                Some(package) => {
                    package.0.push(file);
                }
            }
        }
        Ok(proj)
    }

    pub fn packages_iter(
        &self,
    ) -> std::collections::hash_map::Keys<'_, QualifiedName, FlattenPackage> {
        self.0.keys()
    }
    pub fn contains_package(&self, name: &QualifiedName) -> bool {
        return self.0.contains_key(name);
    }

    pub fn get_package(&self, package_name: &QualifiedName) -> Option<&FlattenPackage> {
        self.0.get(package_name)
    }
}

#[derive(Debug, PartialEq)]
pub struct Package {
    pub name: QualifiedName,
    pub files: Vec<JavaFile>,
}

#[derive(Debug, PartialEq)]
pub(crate) struct FlattenFile(HashMap<QualifiedName, FlattenType>);

impl FlattenFile {
    pub fn contains(&self, fqn: &QualifiedName) -> bool {
        self.0.contains_key(fqn)
    }

    pub fn iter(&self) -> Keys<'_, QualifiedName, FlattenType> {
        self.0.keys()
    }

    pub fn get_type(&self, fqn: &QualifiedName) -> Option<&FlattenType> {
        self.0.get(fqn)
    }
    pub fn from_file(file: &JavaFile) -> Result<Self, ReadProjectErr> {
        let import_objs: Rc<[ImportObject]> = Rc::from(file.imported_objects.clone());
        let mut res: HashMap<QualifiedName, FlattenType> = HashMap::new();
        for typeclass in file.type_decls.iter() {
            let flatten_types = Self::from_type(&typeclass, import_objs.clone());
            for flatten_type in flatten_types.into_iter() {
                if res.contains_key(&flatten_type.name) {
                    return Err(ReadProjectErr::SemanticErr(
                        "duplicate type names in the same file",
                    ));
                } else {
                    res.insert(flatten_type.name.clone(), flatten_type);
                }
            }
        }
        Ok(Self(res))
    }

    /// Flattens a type into a vector of types
    fn from_type(typeclass: &Type, import_objs: Rc<[ImportObject]>) -> Vec<FlattenType> {
        let import_objs: Rc<[ImportObject]> = Rc::from(import_objs);
        Self::recursive_from_type(typeclass, import_objs.clone(), AccessModifier::Public)
    }

    fn recursive_from_type(
        typeclass: &Type,
        import_objs: Rc<[ImportObject]>,
        min_visibility: AccessModifier,
    ) -> Vec<FlattenType> {
        let name = typeclass.name.clone();
        let modifiers = Modifiers {
            modifiers: typeclass.modifiers.modifiers.clone(),
            access_modifier: typeclass.modifiers.access_modifier.min(min_visibility),
        };
        let members: Rc<[Member]> = Rc::from(typeclass.body.members.clone());
        let type_kind = typeclass.type_kind.clone();

        let flatten_type = FlattenType {
            name,
            modifiers: modifiers.clone(),
            members,
            type_kind,
            import_objs: import_objs.clone(),
        };

        let mut res = vec![flatten_type];
        let mut inner_types: Vec<FlattenType> = typeclass
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

#[derive(Debug, PartialEq)]
pub struct FlattenPackage(Vec<FlattenFile>);

impl FlattenPackage {
    pub(crate) fn iter(&self) -> std::slice::Iter<'_, FlattenFile> {
        self.0.iter()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct FlattenType {
    pub name: QualifiedName,
    pub modifiers: Modifiers,
    pub members: Rc<[Member]>,
    pub type_kind: TypeKind,
    pub import_objs: Rc<[ImportObject]>,
}

#[derive(Debug)]
pub struct Scope {
    map: HashMap<QualifiedName, Stack<QualifiedName>>,
}

impl Scope {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn get(&self, name: &QualifiedName) -> Option<QualifiedName> {
        self.map.get(name)?.peek().cloned()
    }
    pub fn push(&mut self, name: &QualifiedName, fqn: &QualifiedName) {
        if self.map.contains_key(name) {
            self.map.get_mut(name).unwrap().push(fqn.clone());
        } else {
            let mut stack = Stack::<QualifiedName>::new();
            stack.push(fqn.clone());
            self.map.insert(name.clone(), stack);
        }
    }

    pub fn pop(&mut self, name: &QualifiedName) -> Option<QualifiedName> {
        if self.map.contains_key(name) && !self.map.get(name).unwrap().is_empty() {
            self.map.get_mut(name).unwrap().pop()
        } else {
            None
        }
    }

    pub fn pop_and_check(&mut self, name: &QualifiedName, fqn: QualifiedName) -> bool {
        self.pop(name).is_some_and(|top_fqn| fqn == top_fqn)
    }

    pub fn pop_uncheck(&mut self, name: &QualifiedName) -> QualifiedName {
        self.map.get_mut(name).unwrap().pop().unwrap()
    }
}

// ---------------------------------------------------------------------------
// ------------------------------------ TEST ---------------------------------
// ---------------------------------------------------------------------------

#[cfg(test)]
mod test {
    use std::str::FromStr;

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
            res.append(&mut FlattenFile::from_type(typeclass, import_objs.clone()));
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

    #[test]
    fn test_flatten_proj() {
        println!(
            "{:#?}",
            FlattenProject::new(PathBuf::from_str("test_target_2").unwrap())
        );
    }
}
