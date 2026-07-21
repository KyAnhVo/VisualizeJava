use std::collections::BTreeSet;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use crate::name_resolution::err::ReadProjectErr;
use crate::types;
use crate::types::AccessModifier;
use crate::types::QualifiedName;

/// A mapping f of type PackageIndex is `f: PackageName -> PackageIndex`.
#[derive(Debug)]
pub struct PackageIndex(HashMap<QualifiedName, TypeIndex>);

impl PackageIndex {
    /// From a list of all AST's in the file, generate a PackageIndex.
    pub fn from_ast_lst(value: &Vec<types::JavaFile>) -> Result<Self, ReadProjectErr> {
        let mut myself = Self(HashMap::new());

        value.iter().try_for_each(|ast| {
            myself
                .0
                .entry(ast.package_name.clone())
                .or_insert(TypeIndex::new(&ast.package_name))
                .add_ast(ast)
        })?;

        Ok(myself)
    }

    /// Iterate over and get pairs of `(pkg name, TypeIndex)`
    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, QualifiedName, TypeIndex> {
        self.0.iter()
    }

    /// Gets the TypeIndex of the file
    pub fn get_package(&self, name: &QualifiedName) -> Option<&TypeIndex> {
        self.0.get(name)
    }

    /// get the package (TypeIndex) containing the type.
    pub fn get_origin_package(&self, name: &QualifiedName) -> Option<&TypeIndex> {
        for i in 1..name.len() {
            let pkg = name.get_prefix(i).unwrap();
            let Some(type_index) = self.get_package(&pkg) else {
                continue;
            };
            let Some(_) = type_index.get_type(name) else {
                continue;
            };
            return Some(type_index);
        }

        None
    }
}

/// A mapping f of type TypeIndex is `f: TypeName (fqn) -> TypeIndexEntry`
#[derive(Debug)]
pub struct TypeIndex {
    pub package: QualifiedName,
    pub type_index: HashMap<QualifiedName, TypeIndexEntry>,
}

impl TypeIndex {
    pub fn new(pkg_name: &QualifiedName) -> Self {
        Self {
            package: pkg_name.clone(),
            type_index: HashMap::new(),
        }
    }

    pub fn get_type(&self, type_name: &QualifiedName) -> Option<&TypeIndexEntry> {
        self.type_index.get(type_name)
    }
    pub fn add_ast(&mut self, ast: &types::JavaFile) -> Result<(), ReadProjectErr> {
        ast.type_decls.iter().try_for_each(|typeclass| {
            self.add_ast_recursive(typeclass, ast.file.clone(), AccessModifier::Public)
        })
    }

    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, QualifiedName, TypeIndexEntry> {
        self.type_index.iter()
    }

    fn add_ast_recursive(
        &mut self,
        typeclass: &types::Type,
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
                name: typeclass.name.clone(),
                visibility,
                from_file: from_file.clone(),
                modifiers: typeclass.modifiers.modifiers.clone(),
            },
        );
        typeclass.body.subtypes.iter().try_for_each(|inner_type| {
            self.add_ast_recursive(inner_type, from_file.clone(), visibility)
        })
    }
}

/// A TypeIndexEntry is a Name with Visibility.
#[derive(Debug)]
pub struct TypeIndexEntry {
    /// the fully qualified name of the type
    pub name: QualifiedName,
    /// the visibility of the type.
    pub visibility: AccessModifier,
    /// the file this type is read from
    pub from_file: Rc<PathBuf>,
    /// the modifiers (static, volatile, etc.)
    pub modifiers: BTreeSet<String>,
}

#[cfg(test)]
pub(crate) mod test {
    use super::*;
    use crate::name_resolution::file_util::get_java_files;
    use crate::parser::parser::Parser;

    /// Parses every .java file under `dir` and builds a PackageIndex from them.
    pub(crate) fn load_project(dir: &str) -> (Vec<types::JavaFile>, PackageIndex) {
        let files = get_java_files(&PathBuf::from(dir)).unwrap();
        let asts: Vec<types::JavaFile> = files
            .iter()
            .map(|file| {
                let src = std::fs::read_to_string(file).unwrap();
                Parser::parse(&src, file).unwrap()
            })
            .collect();
        let project = PackageIndex::from_ast_lst(&asts).unwrap();
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
        let result = PackageIndex::from_ast_lst(&vec![ast1, ast2]);
        assert!(matches!(result, Err(ReadProjectErr::SemanticErr(_))));
    }

    #[test]
    fn test_get_origin_package() {
        let (_asts, project) = load_project("test_target_small");
        let book = QualifiedName(vec!["library".into(), "model".into(), "Book".into()]);
        let origin = project.get_origin_package(&book).unwrap();
        assert_eq!(origin.package, QualifiedName(vec!["library".into(), "model".into()]));

        let list = QualifiedName(vec!["java".into(), "util".into(), "List".into()]);
        assert!(project.get_origin_package(&list).is_none());
    }

    #[test]
    fn test_nested_type_visibility_clamps_to_parent() {
        let src = "package vis.pkg;\npublic class Outer {\n    class Inner {\n    }\n}\n";
        let ast = parse_src(src);
        let project = PackageIndex::from_ast_lst(&vec![ast]).unwrap();
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
