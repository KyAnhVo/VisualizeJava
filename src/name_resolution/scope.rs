use crate::name_resolution::file_util::Stack;
use crate::name_resolution::resolve_types::PackageIndex;
use crate::resolved_types::{self, FullyQualifiedName, PrimitiveType, TypeSource};
use crate::types::{self, AccessModifier, QualifiedName};
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

/// A scope frame is, imagine each time you enter a scope, the new types/names
/// introduced there is called a scope frame.
pub struct ScopeFrame(pub Vec<QualifiedName>);

/// A scope to resolve name with.
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
    fn add_wildcard_import(&mut self, ast: &types::JavaFile, project: &PackageIndex) {
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
                    let Some(pkg) = project.get_origin_package(&import_object.name) else {
                        continue;
                    };
                    // iterate over the type index, get all packages with
                    // prefix is name and is public static
                    for (name, type_index_entry) in pkg.iter() {
                        if !name.has_prefix(&import_object.name) {
                            continue;
                        }
                        if type_index_entry.visibility != AccessModifier::Public {
                            continue;
                        }
                        if !type_index_entry.modifiers.contains(&("static".to_owned())) {
                            continue;
                        }
                        let typename = name.to_type_no_package(&import_object.name).unwrap();
                        self.push(
                            typename,
                            FullyQualifiedName {
                                source: TypeSource::InProjectType {
                                    package: pkg.package.clone(),
                                },
                                typename: type_index_entry.name.clone(),
                            },
                        );
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
                            name.has_prefix(&import_object.name),
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

    fn add_same_pkg(&mut self, ast: &types::JavaFile, project: &PackageIndex) {
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

    fn add_single_type_import(&mut self, ast: &types::JavaFile, project: &PackageIndex) {
        for import_obj in ast.imported_objects.iter() {
            if import_obj.is_wildcard {
                continue;
            }
            let Some(pkg) = project.get_origin_package(&import_obj.name) else {
                continue;
            };
            let Some(entry) = pkg.get_type(&import_obj.name) else {
                continue;
            };
            if entry.visibility != AccessModifier::Public {
                panic!("importing none-public type");
            }
            let typename = QualifiedName(vec![import_obj.name.0.last().unwrap().clone()]);
            self.push(
                typename.clone(),
                FullyQualifiedName {
                    source: TypeSource::InProjectType {
                        package: pkg.package.clone(),
                    },
                    typename: import_obj.name.clone(),
                },
            );
        }
    }

    fn add_same_file(&mut self, ast: &types::JavaFile, project: &PackageIndex) {
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
    pub fn construct_baseline_scope(ast: &types::JavaFile, project: &PackageIndex) -> Self {
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
    fn resolve_type(
        &mut self,
        typeclass: &types::Type,
        pkg_name: &QualifiedName,
        file: Rc<PathBuf>,
    ) -> resolved_types::Type {
        // NOTE: We note that, for `class A extends B implements C, D`, assume all `B, C, D` have an inner
        // type `E` (`E` here can be `E1.E2...En`), then `A` calling `E` would not compile. Thus, since we have been assuming that
        // the project compiles, we can assume that there exists no such `E`, or we error if there
        // are 2 of such `E`.
        //
        // The general algo is as such:
        //  - let GenFrame := Resolve the generic param and get the ScopeFrame
        //  - Resolve and confirm that the extended/implemented type is:
        //      - either same file, or
        //      - at least Default, same package, or
        //      - at least Protected, different package
        //      - else panic.
        //  - let ParentFrame := Collect all inner type names from the types up top
        //    put into scope with the same visibility as above
        //    and put it into scope
        //  - let MyFrame := Collect all inner types and put it into scope (visibility doesn't matter)
        //  - Resolve members
        //  - Recursively resolve inner types
        //  - Pop MyFrame
        //  - Pop ParentFrame
        //  - Pop GenFrame
        unimplemented!();
    }
    fn resolve_member(&mut self, member: &types::Member) -> resolved_types::Member {
        resolved_types::Member {
            name: member.name.clone(),
            annotations: self.resolve_annotations(member.annotations.clone()),
            modifiers: member.modifiers.clone(),
            member_kind: match &member.member_kind {
                types::MemberKind::Property { reftype, arr_dim } => {
                    resolved_types::MemberKind::Property {
                        reftype: self.resolve_reftype(&reftype),
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
                        self.push_and_resolve_type_params(type_param_list);
                    let res = resolved_types::MemberKind::Method {
                        type_param_list: resolve_type_params,
                        input: input
                            .iter()
                            .map(|reftype| self.resolve_reftype(reftype))
                            .collect(),
                        output: self.resolve_voidable_type(output),
                        throws: throws
                            .iter()
                            .map(|reftype| self.resolve_reftype(reftype))
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
                        self.push_and_resolve_type_params(type_param_list);
                    let res = resolved_types::MemberKind::Constructor {
                        type_param_list: resolve_type_params,
                        input: input
                            .iter()
                            .map(|reftype| self.resolve_reftype(reftype))
                            .collect(),
                        throws: throws
                            .iter()
                            .map(|reftype| self.resolve_reftype(reftype))
                            .collect(),
                    };
                    self.pop_frame(&scopeframe);
                    res
                }
            },
        }
    }

    /// Pushes the type param, in, and get the type param for pop
    fn push_and_resolve_type_params(
        &mut self,
        og_type_param_list: &types::TypeParamList,
    ) -> (ScopeFrame, resolved_types::TypeParamList) {
        let mut names: ScopeFrame = ScopeFrame(vec![]);
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
            names.0.push(name.clone());
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
                    .map(|reftype| self.resolve_reftype(reftype))
                    .collect(),
            });
        });

        (names, type_param_list)
    }
    fn resolve_annotations(
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

    fn resolve_voidable_type(
        &self,
        voidable: &types::VoidableType,
    ) -> resolved_types::VoidableType {
        match voidable {
            types::VoidableType::Void => resolved_types::VoidableType::Void,
            types::VoidableType::RefType(s) => {
                resolved_types::VoidableType::RefType(self.resolve_reftype(s))
            }
        }
    }
    fn resolve_reftype(&self, reftype: &types::RefType) -> resolved_types::RefType {
        let name: FullyQualifiedName = match self.peek(&reftype.name) {
            None => match (reftype.name.0.len(), reftype.name.0[0].as_str()) {
                (0, _) => panic!("Type no name"),
                (1, "byte") => FullyQualifiedName {
                    source: TypeSource::PrimitiveType(PrimitiveType::Byte),
                    typename: reftype.name.clone(),
                },
                (1, "short") => FullyQualifiedName {
                    source: TypeSource::PrimitiveType(PrimitiveType::Short),
                    typename: reftype.name.clone(),
                },
                (1, "int") => FullyQualifiedName {
                    source: resolved_types::TypeSource::PrimitiveType(PrimitiveType::Int),
                    typename: reftype.name.clone(),
                },
                (1, "long") => FullyQualifiedName {
                    source: TypeSource::PrimitiveType(PrimitiveType::Long),
                    typename: reftype.name.clone(),
                },
                (1, "float") => FullyQualifiedName {
                    source: TypeSource::PrimitiveType(PrimitiveType::Float),
                    typename: reftype.name.clone(),
                },
                (1, "double") => FullyQualifiedName {
                    source: TypeSource::PrimitiveType(PrimitiveType::Double),
                    typename: reftype.name.clone(),
                },
                (1, "boolean") => FullyQualifiedName {
                    source: TypeSource::PrimitiveType(PrimitiveType::Boolean),
                    typename: reftype.name.clone(),
                },
                (1, "char") => FullyQualifiedName {
                    source: TypeSource::PrimitiveType(PrimitiveType::Char),
                    typename: reftype.name.clone(),
                },
                (_, _) => FullyQualifiedName {
                    source: TypeSource::ExternalDependencyType,
                    typename: reftype.name.clone(),
                },
            },
            Some(s) => s.clone(),
        };

        resolved_types::RefType {
            name,
            type_arg_list: self.resolve_type_arg_list(&reftype.type_arg_list),
            arr_dim: reftype.arr_dim,
        }
    }

    fn resolve_type_arg_list(
        &self,
        typearg_list: &types::TypeArgList,
    ) -> resolved_types::TypeArgList {
        resolved_types::TypeArgList(
            typearg_list
                .0
                .iter()
                .map(|type_arg| self.resolve_type_arg(type_arg))
                .collect(),
        )
    }

    fn resolve_type_arg(&self, typearg: &types::TypeArg) -> resolved_types::TypeArg {
        match typearg {
            types::TypeArg::Is(s) => resolved_types::TypeArg::Is(self.resolve_reftype(s)),
            types::TypeArg::Extends(s) => resolved_types::TypeArg::Extends(self.resolve_reftype(s)),
            types::TypeArg::Super(s) => resolved_types::TypeArg::Super(self.resolve_reftype(s)),
            types::TypeArg::Wildcard => resolved_types::TypeArg::Wildcard,
        }
    }
}
