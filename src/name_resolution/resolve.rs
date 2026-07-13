/** resolve.rs
* The idea of this file is to resolve name for a project
*
* Before the algorithm, we first understand how Java does its name resolution. Here is the order:
* - Type declared in the same file
* - Single type import
* - Type declared in the same package
* - Wildcard import
* - java.lang default import
*
* So the idea is that we have a scope table such that:
* - Scope[name] = (package, typename)
* And we set up in the reverse direction:
* - Put java.lang classes in first
* - Put wildcard imports in
* - Put types declared in the same package in,
* - Put single type imports in
* - Put types declared in the same file in.
* For which put in means fill in if not there, or override if there.
* For example, take the following code:
* ```
* package com.current; // where we have a different file of same package with Character class
* import com.example.util.*; // have a Character class
* import com.example.npc.Character;
* public class Character {...}
* ```
* We consider Scope["Vector"]:
* 1. "Character"
* 2. java.lang.Character
* 3. com.example.util.Character
* 4. com.example.npc.Character;
* 5. com.example.npc.Character; (current file)
* NOTE 1: Although, for package building, we would sweep over the files in the same package
* and verify that no 2 top level classes of a class have the same name so 4 and 5 are
* not necessarily clashing / or raise error right away.
* NOTE 2: Also, we would not consider java.lang since it is over our scope and we do not want
* to draw abstraction/dependency edges to and from java.lang and anything not inside project
* file.
*
*
* Here is the pseudocode for the algo algorithm:
*
* Phase 1: Flatten
* For each file:
* - Recursively flatten the types
* - Put the file with all types at 1st level (so file[type].members is empty) into its
*   corresponding package: Map<Package Name -> Vector<Files>>
*
* Phas 2: Name Resolution
* For each file:
* - First, we construct Scope as described above.
* - Then we do ResolveType recursively.
* ResolveType:
* - Resolve parent type
* - Put generic type of class in Scope
* - Sweep over parent class inner types, put parent class protected/public inner types into scope.
* - Sweep over current class inner types, put them into Scope.
* - For each member (function / variable):
*   - Resolve types for parameters/etc.
* - For each subtype:
*   ResolveSubtype(child)
*/
use crate::name_resolution::err::ReadProjectErr;
use crate::name_resolution::file_util::{Stack, get_java_files_recursive};
use crate::parser;
use crate::types::{
    AccessModifier, ImportObject, JavaFile, Member, Modifiers, QualifiedName, Type, TypeKind,
};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::rc::Rc;
use std::{fs, io};

#[derive(Debug)]
pub(crate) struct FlattenProject(HashMap<QualifiedName, FlattenPackage>);

impl FlattenProject {
    pub(crate) fn new(root_dir: PathBuf) -> Result<Self, ReadProjectErr> {
        let files = get_java_files_recursive(&root_dir.to_path_buf(), &root_dir.to_path_buf())?;
        let mut proj: Self = Self(HashMap::new());
        for (path, _) in files.into_iter() {
            let ast = parser::parser::Parser::parse(fs::read_to_string(path).unwrap().as_str())?;
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
}

#[derive(Debug, PartialEq)]
pub(crate) struct Package {
    pub name: QualifiedName,
    pub files: Vec<JavaFile>,
}

#[derive(Debug, PartialEq)]
pub(crate) struct FlattenFile(HashMap<QualifiedName, FlattenType>);

impl FlattenFile {
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
pub(crate) struct FlattenPackage(Vec<FlattenFile>);
#[derive(Debug, PartialEq)]
pub(crate) struct PackagedTypeName {
    pub package: QualifiedName,
    pub typename: QualifiedName,
}

impl PackagedTypeName {
    /// Returns the fully qualified name of the packaged type
    pub fn fqn(&self) -> QualifiedName {
        let mut v = self.package.0.clone();
        v.extend(self.typename.0.clone());
        QualifiedName(v)
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
struct Scope {
    map: HashMap<QualifiedName, Stack<PackagedTypeName>>,
}

impl Scope {
    fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    fn push(&mut self, name: &QualifiedName, fqn: PackagedTypeName) {
        if self.map.contains_key(name) {
            self.map.get_mut(name).unwrap().push(fqn);
        } else {
            let mut stack = Stack::<PackagedTypeName>::new();
            stack.push(fqn);
            self.map.insert(name.clone(), stack);
        }
    }

    fn pop(&mut self, name: &QualifiedName) -> Option<PackagedTypeName> {
        if self.map.contains_key(name) && !self.map.get(name).unwrap().is_empty() {
            self.map.get_mut(name).unwrap().pop()
        } else {
            None
        }
    }

    fn pop_verify(&mut self, name: &QualifiedName, fqn: PackagedTypeName) -> bool {
        self.pop(name).is_some_and(|top_fqn| fqn == top_fqn)
    }
}

// ---------------------------------------------------------------------------
// ------------------------------------ TEST ---------------------------------
// ---------------------------------------------------------------------------

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
}
