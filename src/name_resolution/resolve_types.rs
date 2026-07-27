use std::collections::BTreeSet;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use crate::name_resolution::err::ReadProjectErr;
use crate::name_resolution::resolve::ExportedInnerTypes;
use crate::resolved_types;
use crate::resolved_types::FullyQualifiedName;
use crate::resolved_types::TypeSource;
use crate::types;
use crate::types::AccessModifier;
use crate::types::QualifiedName;

#[derive(Debug, PartialEq)]
pub enum NameResolutionErr {
    CyclicDependency,
}

/// A mapping f of type PackageIndex is `f: PackageName -> PackageIndex`.
#[derive(Debug)]
pub struct Project(HashMap<QualifiedName, Package>);

impl Project {
    /// From a list of all AST's in the file, generate a PackageIndex.
    pub fn from_ast_lst(value: &[Rc<types::JavaFile>]) -> Result<Self, ReadProjectErr> {
        let mut myself = Self(HashMap::new());

        value.iter().try_for_each(|ast| {
            myself
                .0
                .entry(ast.package_name.clone())
                .or_insert(Package::new(&ast.package_name))
                .add_ast(ast.clone())
        })?;

        Ok(myself)
    }

    /// Iterate over and get pairs of `(pkg name, TypeIndex)`
    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, QualifiedName, Package> {
        self.0.iter()
    }

    /// Gets the TypeIndex of the file
    pub fn get_package(&self, name: &QualifiedName) -> Option<&Package> {
        self.0.get(name)
    }

    pub fn get_mut_package(&mut self, name: &QualifiedName) -> Option<&mut Package> {
        self.0.get_mut(name)
    }

    /// get the Package containing the type and the TypeIndex.
    pub fn get_origin_package(&self, name: &QualifiedName) -> Option<(&Package, &TypeIndexEntry)> {
        for i in 1..name.len() {
            let pkg = name.get_prefix(i).unwrap();
            let Some(type_index) = self.get_package(&pkg) else {
                continue;
            };
            let Some(entry) = type_index.get_type(name) else {
                continue;
            };
            return Some((type_index, entry));
        }

        None
    }

    pub fn to_fqn(&self, fqn: &QualifiedName) -> Rc<[FullyQualifiedName]> {
        let mut v: Vec<FullyQualifiedName> = vec![];
        for i in 0..fqn.len() {
            let pkgname = fqn.get_prefix(i).unwrap();
            let Some(pkg) = self.get_package(&pkgname) else {
                continue;
            };
            let Some(_) = pkg.get_type(fqn) else {
                continue;
            };
            v.push(FullyQualifiedName {
                source: TypeSource::InProjectType {
                    package: pkgname.clone(),
                },
                typename: fqn.clone(),
            });
        }
        v.into()
    }
}

/// A mapping f of type TypeIndex is `f: TypeName (fqn) -> TypeIndexEntry`
#[derive(Debug)]
pub struct Package {
    pub package: QualifiedName,
    pub type_index: HashMap<QualifiedName, TypeIndexEntry>,
}

impl Package {
    pub fn new(pkg_name: &QualifiedName) -> Self {
        Self {
            package: pkg_name.clone(),
            type_index: HashMap::new(),
        }
    }

    pub fn get_type(&self, type_name: &QualifiedName) -> Option<&TypeIndexEntry> {
        self.type_index.get(type_name)
    }

    pub fn get_mut_type(&mut self, type_name: &QualifiedName) -> Option<&mut TypeIndexEntry> {
        self.type_index.get_mut(type_name)
    }
    pub fn add_ast(&mut self, ast: Rc<types::JavaFile>) -> Result<(), ReadProjectErr> {
        ast.type_decls.iter().try_for_each(|typeclass| {
            self.add_ast_recursive(
                typeclass.clone(),
                ast.clone(),
                ast.file.clone(),
                AccessModifier::Public,
            )
        })
    }
    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, QualifiedName, TypeIndexEntry> {
        self.type_index.iter()
    }

    pub fn get_fqn(&self, fqn: &QualifiedName) -> Option<FullyQualifiedName> {
        let Some(_) = self.get_type(fqn) else {
            return None;
        };
        Some(FullyQualifiedName {
            source: TypeSource::InProjectType {
                package: self.package.clone(),
            },
            typename: fqn.clone(),
        })
    }

    fn add_ast_recursive(
        &mut self,
        typeclass: Rc<types::Type>,
        ast_root: Rc<types::JavaFile>,
        from_file: Rc<PathBuf>,
        current_min_access_modifier: AccessModifier,
    ) -> Result<(), ReadProjectErr> {
        let visibility = typeclass
            .modifiers
            .access_modifier
            .min(current_min_access_modifier);

        if self.type_index.contains_key(&typeclass.name) {
            return Err(ReadProjectErr::SemanticErr(
                "duplicated type names inside same package",
            ));
        }
        self.type_index.insert(
            typeclass.name.clone(),
            TypeIndexEntry {
                name: FullyQualifiedName {
                    source: TypeSource::InProjectType {
                        package: ast_root.package_name.clone(),
                    },
                    typename: typeclass.name.clone(),
                },
                visibility,
                from_file: from_file.clone(),
                modifiers: typeclass.modifiers.modifiers.clone(),
                ast_root: ast_root.clone(),
                unresolved_node: typeclass.clone(),
                resolved_node: None,
                export_types: None,
            },
        );
        typeclass.body.subtypes.iter().try_for_each(|inner_type| {
            self.add_ast_recursive(
                inner_type.clone(),
                ast_root.clone(),
                from_file.clone(),
                visibility,
            )
        })
    }
}

/// A TypeIndexEntry is a Name with Visibility.
#[derive(Debug, Clone)]
pub struct TypeIndexEntry {
    // --------------------------- METADATA ----------------------------
    /// the fully qualified name of the type
    pub name: FullyQualifiedName,
    /// the visibility of the type.
    pub visibility: AccessModifier,
    /// the file this type is read from
    pub from_file: Rc<PathBuf>,
    /// the modifiers (static, volatile, etc.)
    pub modifiers: BTreeSet<String>,
    /// the root of the ast that contains this type
    // -------------------------- UNRESOLVED ---------------------------
    pub ast_root: Rc<types::JavaFile>,
    /// the unresolved node of the AST that is this type
    pub unresolved_node: Rc<types::Type>,
    // ------------------------ RESOLVE METADATA -----------------------
    /// the resolved node of the AST that is this type
    pub resolved_node: Option<Rc<resolved_types::Type>>,
    /// the data to propagate to children nodes
    pub export_types: Option<ExportedInnerTypes>,
}

#[cfg(test)]
pub(crate) mod test {
    use super::*;
    use crate::name_resolution::file_util::get_java_files;
    use crate::parser::parser::Parser;

    /// Parses every .java file under `dir` and builds a PackageIndex from them.
    pub(crate) fn load_project(dir: &str) -> (Vec<Rc<types::JavaFile>>, Project) {
        let files = get_java_files(&PathBuf::from(dir)).unwrap();
        let asts: Vec<Rc<types::JavaFile>> = files
            .iter()
            .map(|file| {
                let src = std::fs::read_to_string(file).unwrap();
                Rc::new(Parser::parse(&src, file).unwrap())
            })
            .collect();
        let project = Project::from_ast_lst(&asts).unwrap();
        (asts, project)
    }

    fn parse_src(src: &str) -> types::JavaFile {
        Parser::parse(src, &PathBuf::new()).unwrap()
    }

    #[test]
    fn test_package_count() {
        let (_asts, project) = load_project("test_target_small");
        assert_eq!(project.iter().count(), 7);
    }

    #[test]
    fn test_flatten_recurses_into_nested_types() {
        let (_asts, project) = load_project("test_target_small");
        let pkg = project
            .get_package(&QualifiedName(vec!["library".into(), "model".into()]))
            .unwrap();
        let builder_name = QualifiedName(vec![
            "library".into(),
            "model".into(),
            "Book".into(),
            "Builder".into(),
        ]);
        assert!(pkg.get_type(&builder_name).is_some());
    }

    #[test]
    fn test_duplicate_type_name_errors() {
        let src = "package dup.pkg;\npublic class Foo {\n}\n";
        let ast1 = parse_src(src);
        let ast2 = parse_src(src);
        let result = Project::from_ast_lst(&vec![Rc::new(ast1), Rc::new(ast2)]);
        assert!(matches!(result, Err(ReadProjectErr::SemanticErr(_))));
    }

    #[test]
    fn test_get_origin_package() {
        let (_asts, project) = load_project("test_target_small");
        let book = QualifiedName(vec!["library".into(), "model".into(), "Book".into()]);
        let (origin, _) = project.get_origin_package(&book).unwrap();
        assert_eq!(
            origin.package,
            QualifiedName(vec!["library".into(), "model".into()])
        );

        let list = QualifiedName(vec!["java".into(), "util".into(), "List".into()]);
        assert!(project.get_origin_package(&list).is_none());
    }

    #[test]
    fn test_nested_type_visibility_clamps_to_parent() {
        let src = "package vis.pkg;\npublic class Outer {\n    class Inner {\n    }\n}\n";
        let ast = parse_src(src);
        let project = Project::from_ast_lst(&vec![Rc::new(ast)]).unwrap();
        let pkg = project
            .get_package(&QualifiedName(vec!["vis".into(), "pkg".into()]))
            .unwrap();
        let inner = pkg
            .get_type(&QualifiedName(vec![
                "vis".into(),
                "pkg".into(),
                "Outer".into(),
                "Inner".into(),
            ]))
            .unwrap();
        assert_eq!(inner.visibility, AccessModifier::Default);
    }
}
