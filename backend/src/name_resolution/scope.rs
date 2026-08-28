use crate::name_resolution::export_types::ExportedInnerTypes;
use crate::name_resolution::file_util::Stack;
use crate::name_resolution::resolve::TypeQueueEntry;
use crate::name_resolution::resolve_types::Project;
use crate::resolved_types::{self, FullyQualifiedName, PrimitiveType, TypeSource};
use crate::types::{self, AccessModifier, QualifiedName};
use std::collections::HashMap;
use std::ops::FnOnce;
use std::rc::Rc;

/// A scope frame is, imagine each time you enter a scope, the new types/names
/// introduced there is called a scope frame.
pub struct ScopeFrame(pub Vec<QualifiedName>);

/// A scope to resolve name with.
#[derive(Clone, Debug)]
pub struct Scope(pub HashMap<QualifiedName, Stack<FullyQualifiedName>>);

// -------------------------- Util Functions --------------------------------
impl Scope {
    /// Creates a new Scope with an empty hashmap.
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    /// push the fqn to the stack keyed by the name.
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

    /// pops every item of a frame off.
    pub fn pop_frame(&mut self, frame: &ScopeFrame) {
        frame.0.iter().for_each(|value| {
            self.pop(value);
        });
    }

    /// check if the name is in scope, and get is FQN.
    pub fn get_fqn(&self, name: &QualifiedName) -> Option<&FullyQualifiedName> {
        let stack = self.0.get(name)?;
        stack.peek()
    }
}

// ------------------------ Scope construction at file entry ---------------------
// Notes:
// -    add_same_pkg_import() will add the same file as
//      add_same_file(). The reason we still have add_same_file()
//      is because add_single_type_import might overwrite that scope.
impl Scope {
    fn add_wildcard_import(&mut self, ast: &types::JavaFile, project: &Project) {
        for import_object in ast.imported_objects.iter() {
            // package P; import P.*;
            // just for safety but if anybody do this then touche' to them...
            if import_object.name == ast.package_name {
                continue;
            }
            match (import_object.is_static, import_object.is_wildcard) {
                (true, true) => {
                    // static -> this imples that the name can be
                    // of a type. Thus, we search for the package and
                    // in that package search for all the types that is
                    // in it. Only put in scope the types with the prefix
                    // is import_object.name and public static.

                    // get the type index
                    let Some((pkg, _)) = project.get_origin_package(&import_object.name) else {
                        continue;
                    };
                    // iterate over the type index, get all packages with
                    // prefix is name and is public static
                    for (name, type_index_entry) in pkg.iter() {
                        if !name.has_proper_prefix(&import_object.name) {
                            continue;
                        }
                        if type_index_entry.visibility != AccessModifier::Public {
                            continue;
                        }
                        if !type_index_entry.modifiers.contains(&("static".to_owned())) {
                            continue;
                        }
                        let typename = name.to_type_no_package(&import_object.name).unwrap();
                        self.push(typename, type_index_entry.name.clone());
                    }
                }
                (false, true) => {
                    // Non-static -> this implies that the name
                    // must then be a package.
                    // check that there exists a package with that name.
                    // iterate over the package's types and seperate the
                    // package's name and the type name.
                    let Some(pkg) = project.get_package(&import_object.name) else {
                        // unknown / not-in-project package
                        continue;
                    };
                    for (name, typeclass) in pkg.type_index.iter() {
                        if typeclass.visibility != AccessModifier::Public {
                            continue;
                        }
                        assert!(
                            name.has_proper_prefix(&import_object.name),
                            "type of package does not have package name as prefix"
                        );
                        let typename = name.to_type_no_package(&import_object.name).unwrap();
                        self.push(
                            typename,
                            FullyQualifiedName {
                                source: TypeSource::InProjectType {
                                    package: pkg.package.clone(),
                                },
                                typename: name.clone(),
                            },
                        );
                    }
                }
                _ => {}
            }
        }
    }

    fn add_same_pkg(&mut self, ast: &types::JavaFile, project: &Project) {
        for (fqn, typeclass) in project.get_package(&ast.package_name).unwrap().iter() {
            if typeclass.visibility == AccessModifier::Private {
                continue;
            }
            let typename = fqn.to_type_no_package(&ast.package_name).unwrap();
            self.push(
                typename,
                FullyQualifiedName {
                    source: TypeSource::InProjectType {
                        package: ast.package_name.clone(),
                    },
                    typename: fqn.clone(),
                },
            );
        }
    }

    fn add_single_type_import(&mut self, ast: &types::JavaFile, project: &Project) {
        for import_obj in ast.imported_objects.iter() {
            if import_obj.is_wildcard {
                continue;
            }
            let Some((pkg, _)) = project.get_origin_package(&import_obj.name) else {
                continue;
            };
            for (typename, entry) in pkg.iter() {
                if !typename.has_prefix(&import_obj.name) {
                    continue;
                }
                if entry.visibility != AccessModifier::Public {
                    // justification: we do not need to care about in-package type, since in-package
                    // type is imported right before we do single type import.
                    continue;
                }
                self.push(
                    typename.get_suffix(import_obj.name.len() - 1).unwrap(),
                    FullyQualifiedName {
                        source: TypeSource::InProjectType {
                            package: pkg.package.clone(),
                        },
                        typename: typename.clone(),
                    },
                );
            }
        }
    }

    fn add_same_file(&mut self, ast: &types::JavaFile, project: &Project) {
        for (name, entry) in project.get_package(&ast.package_name).unwrap().iter() {
            if entry.from_file != ast.file {
                continue;
            }
            let typename = name.to_type_no_package(&ast.package_name).unwrap();
            self.push(
                typename,
                FullyQualifiedName {
                    source: TypeSource::InProjectType {
                        package: ast.package_name.clone(),
                    },
                    typename: name.clone(),
                },
            );
        }
    }

    /// Constructs a scope object from a project and an AST.
    /// Refer to name_resolution/README.md to understand how this works.
    pub fn construct_baseline_scope(ast: &types::JavaFile, project: &Project) -> Self {
        let mut scope = Self::new();
        scope.add_wildcard_import(ast, project);
        scope.add_same_pkg(ast, project);
        scope.add_single_type_import(ast, project);
        scope.add_same_file(ast, project);
        scope
    }
}

// ------------------------- Resolving members and types ------------------------
impl Scope {
    pub fn resolve_type_no_child(
        &mut self,
        typeclass: Rc<types::Type>,
        project: &Project,
        ast_root: Rc<types::JavaFile>,
    ) -> (resolved_types::Type, Vec<TypeQueueEntry>) {
        let (frame, type_params) =
            self.push_and_resolve_type_params(&typeclass.type_params, project);
        self.with_frame(frame, |scope| {
            let members: Rc<[Rc<resolved_types::Member>]> = typeclass
                .body
                .members
                .iter()
                .map(|member| scope.resolve_member(member, project).into())
                .collect();
            let children_queue_entries: Vec<TypeQueueEntry> = typeclass
                .body
                .subtypes
                .iter()
                .map(|subtype| TypeQueueEntry {
                    name: subtype.name.clone(),
                    ast_root: ast_root.clone(),
                    type_node: subtype.clone(),
                    type_member_scope: scope.clone(),
                })
                .collect();
            (
                resolved_types::Type {
                    name: scope.resolve_qualified_name(&typeclass.name, project),
                    modifiers: typeclass.modifiers.clone(),
                    annotation: typeclass
                        .annotation
                        .iter()
                        .map(|x| resolved_types::Annotation {
                            name: scope.resolve_qualified_name(&x.name, project),
                            s: x.s.clone(),
                        })
                        .collect(),
                    body: resolved_types::TypeBody {
                        members,
                        subtypes: vec![].into(),
                    },
                    type_kind: scope.resolve_typekind(&typeclass.type_kind, project),
                    type_params,
                }
                .into(),
                children_queue_entries,
            )
        })
    }

    pub fn resolve_typekind(
        &mut self,
        typekind: &types::TypeKind,
        project: &Project,
    ) -> resolved_types::TypeKind {
        match typekind {
            types::TypeKind::Class {
                inherit_class,
                implement_interfaces,
            } => resolved_types::TypeKind::Class {
                inherit_class: match inherit_class.as_ref() {
                    None => None,
                    Some(x) => Some(self.resolve_reftype(x, project)),
                },
                implement_interfaces: implement_interfaces
                    .iter()
                    .map(|x| self.resolve_reftype(x, project))
                    .collect(),
            },
            types::TypeKind::Enum {
                implement_interfaces,
                enum_vals,
            } => resolved_types::TypeKind::Enum {
                implement_interfaces: implement_interfaces
                    .iter()
                    .map(|x| self.resolve_reftype(x, project))
                    .collect(),
                enum_vals: enum_vals.clone(),
            },
            types::TypeKind::Interface { extend_interfaces } => {
                resolved_types::TypeKind::Interface {
                    extend_interfaces: extend_interfaces
                        .iter()
                        .map(|x| self.resolve_reftype(x, project))
                        .collect(),
                }
            }
            types::TypeKind::Annotation {
                annotation_properties,
            } => resolved_types::TypeKind::Annotation {
                annotation_properties: annotation_properties
                    .iter()
                    .map(|(name, reftype)| (name.clone(), self.resolve_reftype(reftype, project)))
                    .collect(),
            },
        }
    }

    pub fn resolve_member(
        &mut self,
        member: &types::Member,
        project: &Project,
    ) -> resolved_types::Member {
        resolved_types::Member {
            name: member.name.clone(),
            annotations: self.resolve_annotations(member.annotations.clone()),
            modifiers: member.modifiers.clone(),
            member_kind: match &member.member_kind {
                types::MemberKind::Property { reftype, arr_dim } => {
                    resolved_types::MemberKind::Property {
                        reftype: self.resolve_reftype(&reftype, project),
                        arr_dim: *arr_dim,
                    }
                }
                types::MemberKind::Method {
                    type_param_list,
                    input,
                    output,
                    throws,
                } => {
                    let (scopeframe, resolve_type_params) =
                        self.push_and_resolve_type_params(type_param_list, project);
                    let res = resolved_types::MemberKind::Method {
                        type_param_list: resolve_type_params,
                        input: input
                            .iter()
                            .map(|reftype| self.resolve_reftype(reftype, project))
                            .collect(),
                        output: self.resolve_voidable_type(output, project),
                        throws: throws
                            .iter()
                            .map(|reftype| self.resolve_reftype(reftype, project))
                            .collect(),
                    };
                    self.pop_frame(&scopeframe);
                    res
                }
                types::MemberKind::Constructor {
                    type_param_list,
                    input,
                    throws,
                } => {
                    let (scopeframe, resolve_type_params) =
                        self.push_and_resolve_type_params(type_param_list, project);
                    let res = resolved_types::MemberKind::Constructor {
                        type_param_list: resolve_type_params,
                        input: input
                            .iter()
                            .map(|reftype| self.resolve_reftype(reftype, project))
                            .collect(),
                        throws: throws
                            .iter()
                            .map(|reftype| self.resolve_reftype(reftype, project))
                            .collect(),
                    };
                    self.pop_frame(&scopeframe);
                    res
                }
            },
        }
    }

    pub fn add_exported_types_from_parent(
        &mut self,
        exported_types: &ExportedInnerTypes,
        pkg_name: &QualifiedName,
    ) -> ScopeFrame {
        use crate::name_resolution::export_types::ExportedTypeEntryName::*;
        use crate::types::AccessModifier::*;
        let mut scopeframe = ScopeFrame(vec![]);
        for (name, entry) in exported_types.0.iter() {
            let min_visibility = if *entry.root_package == *pkg_name {
                Default
            } else {
                Protected
            };
            if entry.visibility < min_visibility {
                continue;
            }
            match &entry.name {
                Inherited(s) => {
                    self.push(name.clone(), s.clone());
                }
                Own(s) => {
                    self.push(name.clone(), s.clone());
                }
                Ambiguous => {
                    self.push(
                        name.clone(),
                        FullyQualifiedName {
                            source: TypeSource::Ambiguous,
                            typename: QualifiedName(vec![]),
                        },
                    );
                }
            }
            scopeframe.0.push(name.clone());
        }
        scopeframe
    }

    /// Pushes the type param, in, and get the type param for pop
    pub fn push_and_resolve_type_params(
        &mut self,
        og_type_param_list: &types::TypeParamList,
        project: &Project,
    ) -> (ScopeFrame, resolved_types::TypeParamList) {
        let mut scope_frame: ScopeFrame = ScopeFrame(vec![]);
        let mut type_param_list: resolved_types::TypeParamList =
            resolved_types::TypeParamList(vec![]);

        // First, Add the type params to the list.
        // Then, resolve the extends(etc.)

        og_type_param_list.0.iter().for_each(|type_param| {
            let name = QualifiedName(vec![type_param.name.clone()]);
            self.push(
                name.clone(),
                FullyQualifiedName {
                    source: TypeSource::Generic,
                    typename: name.clone(),
                },
            );
            scope_frame.0.push(name.clone());
        });

        og_type_param_list.0.iter().for_each(|type_param| {
            type_param_list.0.push(resolved_types::TypeParam {
                name: FullyQualifiedName {
                    source: TypeSource::Generic,
                    typename: QualifiedName(vec![type_param.name.clone()]),
                },
                extends_from: type_param
                    .extends_from
                    .iter()
                    .map(|reftype| self.resolve_reftype(reftype, project))
                    .collect(),
            });
        });

        (scope_frame, type_param_list)
    }
    pub fn resolve_annotations(
        &mut self,
        annotations: Rc<[types::Annotation]>,
    ) -> Rc<[resolved_types::Annotation]> {
        annotations
            .iter()
            .map(|annotation| resolved_types::Annotation {
                name: match self.get_fqn(&annotation.name) {
                    Some(fqn) => fqn.clone(),
                    None => FullyQualifiedName {
                        source: TypeSource::ExternalDependencyType,
                        typename: annotation.name.clone(),
                    },
                },
                s: annotation.s.clone(),
            })
            .collect()
    }

    pub fn resolve_voidable_type(
        &self,
        voidable: &types::VoidableType,
        project: &Project,
    ) -> resolved_types::VoidableType {
        match voidable {
            types::VoidableType::Void => resolved_types::VoidableType::Void,
            types::VoidableType::RefType(s) => {
                resolved_types::VoidableType::RefType(self.resolve_reftype(s, project))
            }
        }
    }
    fn resolve_reftype(
        &self,
        reftype: &types::RefType,
        project: &Project,
    ) -> resolved_types::RefType {
        resolved_types::RefType {
            name: self.resolve_qualified_name(&reftype.name, project),
            type_arg_list: self.resolve_type_arg_list(&reftype.type_arg_list, project),
            arr_dim: reftype.arr_dim,
        }
    }

    fn resolve_type_arg_list(
        &self,
        typearg_list: &types::TypeArgList,
        project: &Project,
    ) -> resolved_types::TypeArgList {
        resolved_types::TypeArgList(
            typearg_list
                .0
                .iter()
                .map(|type_arg| self.resolve_type_arg(type_arg, project))
                .collect(),
        )
    }
    fn resolve_type_arg(
        &self,
        typearg: &types::TypeArg,
        project: &Project,
    ) -> resolved_types::TypeArg {
        match typearg {
            types::TypeArg::Is(s) => resolved_types::TypeArg::Is(self.resolve_reftype(s, project)),
            types::TypeArg::Extends(s) => {
                resolved_types::TypeArg::Extends(self.resolve_reftype(s, project))
            }
            types::TypeArg::Super(s) => {
                resolved_types::TypeArg::Super(self.resolve_reftype(s, project))
            }
            types::TypeArg::Wildcard => resolved_types::TypeArg::Wildcard,
        }
    }

    /// In general, find an FQN for any QN.
    pub fn resolve_qualified_name(
        &self,
        name: &QualifiedName,
        project: &Project,
    ) -> FullyQualifiedName {
        if let Some(typename) = self.peek(name) {
            typename.clone()
        } else if let Some((_, typename)) = project.get_origin_package(name) {
            typename.name.clone()
        } else {
            match (name.0.len(), name.0[0].as_str()) {
                (0, _) => panic!("Type no name"),
                (1, "byte") => FullyQualifiedName {
                    source: TypeSource::PrimitiveType(PrimitiveType::Byte),
                    typename: name.clone(),
                },
                (1, "short") => FullyQualifiedName {
                    source: TypeSource::PrimitiveType(PrimitiveType::Short),
                    typename: name.clone(),
                },
                (1, "int") => FullyQualifiedName {
                    source: resolved_types::TypeSource::PrimitiveType(PrimitiveType::Int),
                    typename: name.clone(),
                },
                (1, "long") => FullyQualifiedName {
                    source: TypeSource::PrimitiveType(PrimitiveType::Long),
                    typename: name.clone(),
                },
                (1, "float") => FullyQualifiedName {
                    source: TypeSource::PrimitiveType(PrimitiveType::Float),
                    typename: name.clone(),
                },
                (1, "double") => FullyQualifiedName {
                    source: TypeSource::PrimitiveType(PrimitiveType::Double),
                    typename: name.clone(),
                },
                (1, "boolean") => FullyQualifiedName {
                    source: TypeSource::PrimitiveType(PrimitiveType::Boolean),
                    typename: name.clone(),
                },
                (1, "char") => FullyQualifiedName {
                    source: TypeSource::PrimitiveType(PrimitiveType::Char),
                    typename: name.clone(),
                },
                (_, _) => FullyQualifiedName {
                    source: TypeSource::ExternalDependencyType,
                    typename: name.clone(),
                },
            }
        }
    }

    pub fn with_frame<T>(&mut self, frame: ScopeFrame, f: impl FnOnce(&mut Self) -> T) -> T {
        let res = f(self);
        self.pop_frame(&frame);
        res
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::name_resolution::resolve_types::test::load_project;

    /// Finds the AST for a specific fixture file by package + file name.
    /// The project-wide `load_project` helper loads every file under the
    /// fixture directory at once, so individual tests pick out the one
    /// file whose `imported_objects`/`package_name` they want to exercise.
    fn find_ast(
        asts: &[Rc<types::JavaFile>],
        package: &[&str],
        file_name: &str,
    ) -> Rc<types::JavaFile> {
        asts.iter()
            .find(|ast| {
                ast.package_name.0
                    == package
                        .iter()
                        .map(|s| s.to_string())
                        .collect::<Vec<String>>()
                    && ast.file.file_name().and_then(|f| f.to_str()) == Some(file_name)
            })
            .unwrap_or_else(|| panic!("fixture file {file_name} not found in package {package:?}"))
            .clone()
    }

    fn qn(parts: &[&str]) -> QualifiedName {
        QualifiedName(parts.iter().map(|s| s.to_string()).collect())
    }

    /// Builds the expected FullyQualifiedName for an in-project type,
    /// where `typename_suffix` is relative to `package`.
    fn fqn_in_project(package: &[&str], typename_suffix: &[&str]) -> FullyQualifiedName {
        let mut typename: Vec<String> = package.iter().map(|s| s.to_string()).collect();
        typename.extend(typename_suffix.iter().map(|s| s.to_string()));
        FullyQualifiedName {
            source: TypeSource::InProjectType {
                package: qn(package),
            },
            typename: QualifiedName(typename),
        }
    }

    // --------------------------- add_wildcard_import ---------------------------

    /// `import scope.wildcard.*;` should bind every public type in the target
    /// package -- including multi-level nested ones, keyed relative to the
    /// package -- while skipping package-private types entirely.
    #[test]
    fn wildcard_import_public() {
        let (asts, project) = load_project("test_target/scope");
        let ast = find_ast(&asts, &["scope", "consumer"], "WildcardConsumer.java");

        let mut scope = Scope::new();
        scope.add_wildcard_import(&ast, &project);

        assert_eq!(
            scope.get_fqn(&qn(&["Widget"])),
            Some(&fqn_in_project(&["scope", "wildcard"], &["Widget"]))
        );
        assert_eq!(
            scope.get_fqn(&qn(&["Nested"])),
            Some(&fqn_in_project(&["scope", "wildcard"], &["Nested"]))
        );
        assert_eq!(
            scope.get_fqn(&qn(&["Nested", "Child"])),
            Some(&fqn_in_project(
                &["scope", "wildcard"],
                &["Nested", "Child"]
            ))
        );
        // package-private types are not brought in by `import P.*;`
        assert!(scope.get_fqn(&qn(&["Helper"])).is_none());
    }

    /// `import static scope.pkg.Outer.*;` should bind Outer's own public
    /// static nested types keyed relative to Outer itself (not the package,
    /// and not Outer), and still exclude private nested types.
    #[test]
    fn wildcard_import_static() {
        let (asts, project) = load_project("test_target/scope");
        let ast = find_ast(&asts, &["scope", "consumer"], "StaticWildcardConsumer.java");

        let mut scope = Scope::new();
        scope.add_wildcard_import(&ast, &project);

        // `import static scope.pkg.Outer.*;` binds Outer's own public static
        // nested types by their name relative to Outer, not to the package.
        assert_eq!(
            scope.get_fqn(&qn(&["Inner"])),
            Some(&fqn_in_project(&["scope", "pkg"], &["Outer", "Inner"]))
        );
        assert_eq!(
            scope.get_fqn(&qn(&["Inner", "Deep"])),
            Some(&fqn_in_project(
                &["scope", "pkg"],
                &["Outer", "Inner", "Deep"]
            ))
        );
        // Outer itself is not a member of Outer.
        assert!(scope.get_fqn(&qn(&["Outer"])).is_none());
        // private nested types are excluded even under a static wildcard import.
        assert!(scope.get_fqn(&qn(&["Hidden"])).is_none());
    }

    // ----------------------------- add_same_pkg ---------------------------------

    /// Types declared in the same package as the current file should be in
    /// scope without an import, including default (package-private) access
    /// and nested chains, but private nested types stay excluded.
    #[test]
    fn same_pkg_scope() {
        let (asts, project) = load_project("test_target/scope");
        let ast = find_ast(&asts, &["scope", "pkg"], "SamePkgConsumer.java");

        let mut scope = Scope::new();
        scope.add_same_pkg(&ast, &project);

        assert_eq!(
            scope.get_fqn(&qn(&["Sibling"])),
            Some(&fqn_in_project(&["scope", "pkg"], &["Sibling"]))
        );
        // default (package-private) access is visible within the same package.
        assert_eq!(
            scope.get_fqn(&qn(&["PackagePrivate"])),
            Some(&fqn_in_project(&["scope", "pkg"], &["PackagePrivate"]))
        );
        assert_eq!(
            scope.get_fqn(&qn(&["Outer", "Inner", "Deep"])),
            Some(&fqn_in_project(
                &["scope", "pkg"],
                &["Outer", "Inner", "Deep"]
            ))
        );
        // private nested types are excluded even from the declaring type's own package.
        assert!(scope.get_fqn(&qn(&["Outer", "Hidden"])).is_none());
    }

    // ------------------------- add_single_type_import ---------------------------

    /// `import scope.pkg.Outer;` should bind Outer itself and its full nested
    /// chain, each keyed with `Outer` as the leading segment (i.e. relative
    /// to the package, not to the imported name).
    #[test]
    fn single_import_top_level() {
        let (asts, project) = load_project("test_target/scope");
        let ast = find_ast(&asts, &["scope", "consumer"], "SingleImportOuter.java");

        let mut scope = Scope::new();
        scope.add_single_type_import(&ast, &project);

        // `import scope.pkg.Outer;` binds Outer itself...
        assert_eq!(
            scope.get_fqn(&qn(&["Outer"])),
            Some(&fqn_in_project(&["scope", "pkg"], &["Outer"]))
        );
        // ...and its nested descendants, keyed relative to the package (Outer stays
        // the leading segment) rather than relative to the imported name itself.
        assert_eq!(
            scope.get_fqn(&qn(&["Outer", "Inner"])),
            Some(&fqn_in_project(&["scope", "pkg"], &["Outer", "Inner"]))
        );
        assert_eq!(
            scope.get_fqn(&qn(&["Outer", "Inner", "Deep"])),
            Some(&fqn_in_project(
                &["scope", "pkg"],
                &["Outer", "Inner", "Deep"]
            ))
        );
        assert!(scope.get_fqn(&qn(&["Outer", "Hidden"])).is_none());
    }

    /// `import scope.pkg.Outer.Inner;` should bind the bare name `Inner`
    /// (plus its own nested chain), not `Outer.Inner` -- regression test for
    /// the `import_obj.name.len() - 1` suffix fix; using the package length
    /// instead would wrongly keep the `Outer.` prefix here.
    #[test]
    fn single_import_nested() {
        let (asts, project) = load_project("test_target/scope");
        let ast = find_ast(&asts, &["scope", "consumer"], "SingleImportNested.java");

        let mut scope = Scope::new();
        scope.add_single_type_import(&ast, &project);

        // `import scope.pkg.Outer.Inner;` must bind the bare name `Inner`,
        // not `Outer.Inner` -- Outer was never imported here.
        assert_eq!(
            scope.get_fqn(&qn(&["Inner"])),
            Some(&fqn_in_project(&["scope", "pkg"], &["Outer", "Inner"]))
        );
        assert_eq!(
            scope.get_fqn(&qn(&["Inner", "Deep"])),
            Some(&fqn_in_project(
                &["scope", "pkg"],
                &["Outer", "Inner", "Deep"]
            ))
        );
        assert!(scope.get_fqn(&qn(&["Outer"])).is_none());
        assert!(scope.get_fqn(&qn(&["Outer", "Inner"])).is_none());
    }

    // ---------------------------- add_same_file ----------------------------------

    /// A type declared in the current file should be bound in scope under
    /// its own simple name.
    #[test]
    fn same_file_scope() {
        let (asts, project) = load_project("test_target/scope");
        let ast = find_ast(&asts, &["scope", "consumer"], "Character.java");

        let mut scope = Scope::new();
        scope.add_same_file(&ast, &project);

        assert_eq!(
            scope.get_fqn(&qn(&["Character"])),
            Some(&fqn_in_project(&["scope", "consumer"], &["Character"]))
        );
    }

    // ------------------------- precedence / shadowing -----------------------------

    /// For the same simple name, a single type import should overwrite a
    /// wildcard import's binding rather than being shadowed by it -- checked
    /// in isolation from the same-file tier.
    #[test]
    fn import_precedence() {
        let (asts, project) = load_project("test_target/scope");
        let ast = find_ast(&asts, &["scope", "consumer"], "Character.java");

        let mut scope = Scope::new();
        scope.add_wildcard_import(&ast, &project);
        assert_eq!(
            scope.get_fqn(&qn(&["Character"])),
            Some(&fqn_in_project(&["scope", "shadow1"], &["Character"]))
        );

        scope.add_single_type_import(&ast, &project);
        assert_eq!(
            scope.get_fqn(&qn(&["Character"])),
            Some(&fqn_in_project(&["scope", "shadow2"], &["Character"]))
        );
    }

    /// End-to-end version of the README's worked shadowing example: a
    /// wildcard import, a single type import, and a same-file declaration
    /// all bind the same simple name `Character`. Per the documented
    /// precedence order, the type declared in the current file must win.
    ///
    /// ```java
    /// package scope.consumer;
    /// import scope.shadow1.*;         // wildcard import: shadow1.Character
    /// import scope.shadow2.Character; // single type import: shadow2.Character
    /// public class Character {}       // declared in this very file
    /// ```
    #[test]
    fn shadowing_example() {
        let (asts, project) = load_project("test_target/scope");
        let ast = find_ast(&asts, &["scope", "consumer"], "Character.java");

        let scope = Scope::construct_baseline_scope(&ast, &project);

        assert_eq!(
            scope.get_fqn(&qn(&["Character"])),
            Some(&fqn_in_project(&["scope", "consumer"], &["Character"]))
        );
    }

    // --------------------------- resolve_qualified_name ---------------------------

    /// A primitive name like `int` should resolve to its `PrimitiveType`
    /// regardless of scope or project contents.
    #[test]
    fn resolve_primitive() {
        let (asts, project) = load_project("test_target/scope");
        let ast = find_ast(&asts, &["scope", "consumer"], "FullyQualifiedConsumer.java");
        let scope = Scope::construct_baseline_scope(&ast, &project);

        let resolved = scope.resolve_qualified_name(&qn(&["int"]), &project);
        assert_eq!(
            resolved.source,
            TypeSource::PrimitiveType(PrimitiveType::Int)
        );
    }

    /// A name that is neither in scope, nor a project type, nor a primitive
    /// should be classified as an external dependency rather than erroring.
    #[test]
    fn resolve_external() {
        let (asts, project) = load_project("test_target/scope");
        let ast = find_ast(&asts, &["scope", "consumer"], "FullyQualifiedConsumer.java");
        let scope = Scope::construct_baseline_scope(&ast, &project);

        let resolved = scope.resolve_qualified_name(&qn(&["java", "util", "List"]), &project);
        assert_eq!(resolved.source, TypeSource::ExternalDependencyType);
    }

    /// A name only reachable through an import binding (not a real package
    /// path) should resolve via the scope lookup, not the project fallback.
    #[test]
    fn resolve_scope_hit() {
        let (asts, project) = load_project("test_target/scope");
        let ast = find_ast(&asts, &["scope", "consumer"], "SingleImportNested.java");
        let scope = Scope::construct_baseline_scope(&ast, &project);

        // `Inner` is only reachable through the scope binding created by the
        // single type import of `scope.pkg.Outer.Inner`.
        let resolved = scope.resolve_qualified_name(&qn(&["Inner"]), &project);
        assert_eq!(
            resolved,
            fqn_in_project(&["scope", "pkg"], &["Outer", "Inner"])
        );
    }

    /// Writing a name fully qualified, with no matching import at all,
    /// should still resolve by falling back to the project index once the
    /// scope lookup misses.
    #[test]
    fn resolve_fqn_fallback() {
        let (asts, project) = load_project("test_target/scope");
        let ast = find_ast(&asts, &["scope", "consumer"], "FullyQualifiedConsumer.java");
        let scope = Scope::construct_baseline_scope(&ast, &project);

        // Nothing imports scope.pkg.Outer.Inner here, but writing the fully
        // qualified name resolves via the project index once scope misses.
        let resolved =
            scope.resolve_qualified_name(&qn(&["scope", "pkg", "Outer", "Inner"]), &project);
        assert_eq!(
            resolved,
            fqn_in_project(&["scope", "pkg"], &["Outer", "Inner"])
        );
    }
}
