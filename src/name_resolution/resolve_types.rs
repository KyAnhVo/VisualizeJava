use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use crate::name_resolution::err::ReadProjectErr;
use crate::name_resolution::file_util::Stack;
use crate::resolved_types::FullyQualifiedName;
use crate::types;
use crate::types::AccessModifier;
use crate::types::QualifiedName;

/// A mapping f of type PackageIndex is `f: PackageName -> PackageIndex`.
pub struct PackageIndex(pub HashMap<QualifiedName, TypeIndex>);

impl PackageIndex {
    fn from_ast_lst(value: Vec<types::JavaFile>) -> Result<Self, ReadProjectErr> {
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
}

/// A mapping f of type TypeIndex is `f: TypeName (fqn) -> TypeIndexEntry`
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
    pub fn add_ast(&mut self, ast: &types::JavaFile) -> Result<(), ReadProjectErr> {
        ast.type_decls.iter().try_for_each(|typeclass| {
            self.add_ast_recursive(typeclass.clone(), ast.file.clone(), AccessModifier::Public)
        })
    }

    fn add_ast_recursive(
        &mut self,
        typeclass: Rc<types::Type>,
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
            },
        );
        typeclass.body.subtypes.iter().try_for_each(|inner_type| {
            self.add_ast_recursive(inner_type.clone(), from_file.clone(), visibility)
        })
    }
}

/// A TypeIndexEntry is a Name with Visibility.
pub struct TypeIndexEntry {
    /// the fully qualified name of the type
    pub name: QualifiedName,
    /// the visibility of the type. Not private.
    pub visibility: AccessModifier,
    /// the file this type is read from
    pub from_file: Rc<PathBuf>,
}

/// A scope to resolve name with.
pub struct Scope(pub HashMap<QualifiedName, Stack<FullyQualifiedName>>);

impl Scope {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn push(&mut self, name: QualifiedName, fqn: FullyQualifiedName) {
        self.0
            .entry(name.clone())
            .or_insert(Stack::new())
            .push(fqn.clone());
    }

    /// pop returns the fqn in question, or None if
    /// key is not there or the scope for that name is empty, either
    /// way an error.
    pub fn pop(&mut self, name: &QualifiedName) -> Option<FullyQualifiedName> {
        self.0.get_mut(name)?.pop()
    }

    pub fn peek(&self, name: &QualifiedName) -> Option<&FullyQualifiedName> {
        self.0.get(name)?.peek()
    }
}

