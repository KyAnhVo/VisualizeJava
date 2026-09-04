use std::{collections::VecDeque, rc::Rc};

use crate::{
    name_resolution::{
        err::ReadProjectErr,
        export_types::ExportedInnerTypes,
        resolve_types::{NameResolutionErr, Package, Project},
        scope::Scope,
    },
    resolved_types::{self, TypeSource},
    types::{self, QualifiedName},
};

#[derive(Debug, Clone)]
pub struct TypeQueueEntry {
    pub name: QualifiedName,
    pub ast_root: Rc<types::JavaFile>,
    pub type_node: Rc<types::Type>,
    pub type_member_scope: Scope,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolveStatus {
    Finished,
    Unfinished,
}

#[derive(Debug)]
pub struct Resolver {
    pub queue: VecDeque<TypeQueueEntry>,
    pub project: Project,
    early_termination_counter: usize,
}

impl Resolver {
    pub fn new(asts: &[Rc<types::JavaFile>]) -> Result<Self, ReadProjectErr> {
        let project = Project::from_ast_lst(&asts)?;
        let mut me: Self = Self {
            queue: VecDeque::new(),
            project,
            early_termination_counter: 0,
        };
        for ast in asts.iter() {
            let scope = Scope::construct_baseline_scope(ast, &me.project);
            for top_level_type in ast.type_decls.iter() {
                me.queue.push_back(TypeQueueEntry {
                    name: top_level_type.name.clone(),
                    ast_root: ast.clone(),
                    type_node: top_level_type.clone(),
                    type_member_scope: scope.clone(),
                });
            }
        }
        Ok(me)
    }

    pub fn resolve(asts: &[Rc<types::JavaFile>]) -> Rc<[Rc<resolved_types::FileTypeTree>]> {
        // 1. create a new Resolver.
        let mut resolver = Self::new(asts).unwrap();
        // 2. continuously dequeue and resolve
        while resolver.deque_and_resolve().unwrap() == ResolveStatus::Unfinished {}
        // 3. construct the trees from the ground up.
        asts.iter()
            .map(|x| resolver.construct_tree(x.clone()))
            .collect()
    }

    pub fn construct_tree(
        &mut self,
        node: Rc<types::JavaFile>,
    ) -> Rc<resolved_types::FileTypeTree> {
        let pkg = self.project.get_mut_package(&node.package_name).unwrap();
        Rc::new(resolved_types::FileTypeTree(
            node.type_decls
                .iter()
                .map(|typeclass_rc| Self::construct_tree_recursive(typeclass_rc.clone(), pkg))
                .collect(),
        ))
    }

    fn construct_tree_recursive(
        type_node: Rc<types::Type>,
        pkg: &mut Package,
    ) -> Rc<resolved_types::Type> {
        let subtypes: Vec<Rc<resolved_types::Type>> = type_node
            .body
            .subtypes
            .iter()
            .map(|subtype| Self::construct_tree_recursive(subtype.clone(), pkg))
            .collect();
        let type_index = pkg.get_mut_type(&type_node.name).unwrap();
        let mut resolved_type_node = type_index.resolved_node.take().unwrap();
        resolved_type_node.body.subtypes = subtypes.into();
        Rc::new(resolved_type_node)
    }

    pub fn deque_and_resolve(&mut self) -> Result<ResolveStatus, NameResolutionErr> {
        if self.queue.is_empty() {
            return Ok(ResolveStatus::Finished);
        }
        if self.queue.len() == self.early_termination_counter {
            return Err(NameResolutionErr::CyclicDependency);
        }

        // resolve parent. If parent is resolved, then resolve it.
        // Else push it back to the queue.
        let entry = self.queue.pop_front().unwrap();
        if self.entry_parents_resolved(&entry, &self.project) {
            self.early_termination_counter = 0;
            self.resolve_entry(entry);
        } else {
            self.early_termination_counter += 1;
            self.queue.push_back(entry);
        }

        Ok(ResolveStatus::Unfinished)
    }

    fn resolve_entry(&mut self, mut entry: TypeQueueEntry) {
        // we assume parents are resolved.
        let scope = &mut entry.type_member_scope;
        let pkg_name = &entry.ast_root.package_name;

        // Get scope from parent
        let entry_exported_types =
            ExportedInnerTypes::from_type(entry.type_node.clone(), scope, &self.project);
        let inheritance_frame =
            scope.add_exported_types_from_parent(&entry_exported_types, pkg_name);
        self.project
            .get_mut_package(pkg_name)
            .unwrap()
            .get_mut_type(&entry.name)
            .unwrap()
            .export_types = Some(entry_exported_types);

        let (resolved_node, children_subtypes) = scope.with_frame(inheritance_frame, |scope| {
            scope.resolve_type_no_child(
                entry.type_node.clone(),
                &self.project,
                entry.ast_root.clone(),
            )
        });
        self.project
            .get_mut_package(pkg_name)
            .unwrap()
            .get_mut_type(&entry.name)
            .unwrap()
            .resolved_node = Some(resolved_node);
        self.queue.extend(children_subtypes.into_iter());
    }

    fn entry_parents_resolved(&self, entry: &TypeQueueEntry, project: &Project) -> bool {
        match &entry.type_node.type_kind {
            types::TypeKind::Class {
                inherit_class,
                implement_interfaces,
            } => {
                inherit_class
                    .as_ref()
                    .is_none_or(|reftype| self.reftype_is_resolved(&entry, &reftype, project))
                    && implement_interfaces
                        .iter()
                        .all(|reftype| self.reftype_is_resolved(&entry, reftype, project))
            }
            types::TypeKind::Enum {
                implement_interfaces,
                ..
            } => implement_interfaces
                .iter()
                .all(|reftype| self.reftype_is_resolved(&entry, reftype, project)),
            types::TypeKind::Interface { extend_interfaces } => extend_interfaces
                .iter()
                .all(|reftype| self.reftype_is_resolved(&entry, reftype, project)),
            types::TypeKind::Annotation { .. } => true,
        }
    }
    fn reftype_is_resolved(
        &self,
        entry: &TypeQueueEntry,
        reftype: &types::RefType,
        project: &Project,
    ) -> bool {
        use TypeSource::*;
        let fqn = entry
            .type_member_scope
            .resolve_qualified_name(&reftype.name, project);
        match fqn.source {
            InProjectType { package } => self.name_is_resolved(&fqn.typename, &package),
            Ambiguous => panic!("Ambiguous resolution for parent"),
            PrimitiveType(_) => panic!("Attempting to extend/implement primitive types"),
            ExternalDependencyType => true,
            Generic => true,
        }
    }

    fn name_is_resolved(&self, name: &QualifiedName, pkg: &QualifiedName) -> bool {
        self.project
            .get_package(pkg)
            .unwrap()
            .get_type(name)
            .unwrap()
            .resolved_node
            .is_some()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::name_resolution::export_types::ExportedTypeEntryName;
    use crate::name_resolution::resolve_types::test::load_project;

    fn qn(parts: &[&str]) -> QualifiedName {
        QualifiedName(parts.iter().map(|s| s.to_string()).collect())
    }

    fn fqn_in_project(package: &[&str], typename: &[&str]) -> resolved_types::FullyQualifiedName {
        resolved_types::FullyQualifiedName {
            source: TypeSource::InProjectType {
                package: qn(package),
            },
            typename: qn(typename),
        }
    }

    /// Parses `test_target/name_resolution` and drives the Resolver to
    /// completion. Returns the parsed ASTs alongside it (needed by
    /// `construct_tree`) rather than just the final `FileTypeTree`s, so
    /// tests can inspect `Resolver.project` directly -- both
    /// `resolved_node` and `export_types` (which never makes it into the
    /// final tree at all).
    fn resolve_fixture() -> (Vec<Rc<types::JavaFile>>, Resolver) {
        let (asts, _project) = load_project("test_target/name_resolution");
        let mut resolver = Resolver::new(&asts).unwrap();
        while resolver.deque_and_resolve().unwrap() == ResolveStatus::Unfinished {}
        (asts, resolver)
    }

    fn find_ast(
        asts: &[Rc<types::JavaFile>],
        package: &[&str],
        file_name: &str,
    ) -> Rc<types::JavaFile> {
        asts.iter()
            .find(|ast| {
                ast.package_name == qn(package)
                    && ast.file.file_name().and_then(|f| f.to_str()) == Some(file_name)
            })
            .unwrap_or_else(|| panic!("fixture file {file_name} not found in package {package:?}"))
            .clone()
    }

    fn resolved<'a>(
        resolver: &'a Resolver,
        pkg: &[&str],
        name: &[&str],
    ) -> &'a resolved_types::Type {
        resolver
            .project
            .get_package(&qn(pkg))
            .unwrap()
            .get_type(&qn(name))
            .unwrap()
            .resolved_node
            .as_ref()
            .unwrap()
    }

    fn member<'a>(t: &'a resolved_types::Type, name: &str) -> &'a resolved_types::Member {
        t.body
            .members
            .iter()
            .find(|m| m.name == name)
            .unwrap_or_else(|| panic!("member {name} not found"))
    }

    fn property_type(m: &resolved_types::Member) -> &resolved_types::FullyQualifiedName {
        match &m.member_kind {
            resolved_types::MemberKind::Property { reftype, .. } => &reftype.name,
            _ => panic!("expected a property member"),
        }
    }

    /// Import precedence: a wildcard import, a single-type import, and a
    /// same-file declaration all bind the simple name "Token". The type
    /// declared in the current file must win.
    #[test]
    fn import_shadowing_prefers_same_file() {
        let (_asts, resolver) = resolve_fixture();
        let token = resolved(&resolver, &["nres", "consumer"], &["nres", "consumer", "Token"]);
        assert_eq!(
            token.name,
            fqn_in_project(&["nres", "consumer"], &["nres", "consumer", "Token"])
        );
    }

    /// A plain wildcard import reaches multi-level nested public types,
    /// each still qualified by their enclosing type.
    #[test]
    fn wildcard_import_reaches_nested_types() {
        let (_asts, resolver) = resolve_fixture();
        let consumer = resolved(
            &resolver,
            &["nres", "consumer"],
            &["nres", "consumer", "GadgetConsumer"],
        );
        assert_eq!(
            property_type(member(consumer, "gadget")),
            &fqn_in_project(&["nres", "wildcard"], &["nres", "wildcard", "Gadget"])
        );
        assert_eq!(
            property_type(member(consumer, "part")),
            &fqn_in_project(
                &["nres", "wildcard"],
                &["nres", "wildcard", "Gadget", "Part"]
            )
        );
        assert_eq!(
            property_type(member(consumer, "subPart")),
            &fqn_in_project(
                &["nres", "wildcard"],
                &["nres", "wildcard", "Gadget", "Part", "SubPart"]
            )
        );
    }

    /// A static wildcard import binds the target type's own nested types
    /// relative to itself (bare "Color"), not relative to the package.
    #[test]
    fn static_wildcard_import_binds_nested_type_bare() {
        let (_asts, resolver) = resolve_fixture();
        let consumer = resolved(
            &resolver,
            &["nres", "consumer"],
            &["nres", "consumer", "ThemeConsumer"],
        );
        assert_eq!(
            property_type(member(consumer, "color")),
            &fqn_in_project(
                &["nres", "staticimport"],
                &["nres", "staticimport", "Palette", "Color"]
            )
        );
    }

    /// A self-referencing generic bound (`T extends SelfComparable<T>`)
    /// resolves the bound to the enclosing type itself, with the type-arg
    /// staying a `Generic` reference to `T`.
    #[test]
    fn self_referencing_generic_bound_resolves() {
        let (_asts, resolver) = resolve_fixture();
        let iface = resolved(
            &resolver,
            &["nres", "generics"],
            &["nres", "generics", "SelfComparable"],
        );
        let bound = &iface.type_params.0[0].extends_from[0];
        assert_eq!(
            bound.name,
            fqn_in_project(
                &["nres", "generics"],
                &["nres", "generics", "SelfComparable"]
            )
        );
        assert_eq!(
            bound.type_arg_list.0[0],
            resolved_types::TypeArg::Is(resolved_types::RefType {
                name: resolved_types::FullyQualifiedName {
                    source: TypeSource::Generic,
                    typename: qn(&["T"]),
                },
                type_arg_list: resolved_types::TypeArgList(vec![]),
                arr_dim: 0,
            })
        );
    }

    /// Mutually referencing bounds (`T extends Identifiable2<ID>, ID`) --
    /// `T`'s bound refers to `ID`, declared after it in the same list.
    /// Resolving this correctly requires both type params to be pushed into
    /// scope before either bound is resolved.
    #[test]
    fn mutually_referencing_generic_bounds_resolve() {
        let (_asts, resolver) = resolve_fixture();
        let store = resolved(
            &resolver,
            &["nres", "generics"],
            &["nres", "generics", "Store"],
        );
        assert_eq!(store.type_params.0.len(), 2);

        let t_bound = &store.type_params.0[0].extends_from[0];
        assert_eq!(
            t_bound.name,
            fqn_in_project(
                &["nres", "generics"],
                &["nres", "generics", "Identifiable2"]
            )
        );
        assert_eq!(
            t_bound.type_arg_list.0[0],
            resolved_types::TypeArg::Is(resolved_types::RefType {
                name: resolved_types::FullyQualifiedName {
                    source: TypeSource::Generic,
                    typename: qn(&["ID"]),
                },
                type_arg_list: resolved_types::TypeArgList(vec![]),
                arr_dim: 0,
            })
        );
        assert!(store.type_params.0[1].extends_from.is_empty());
    }

    /// A two-hop inheritance chain: `WidgetStore extends Repo<Widget,
    /// String>`, where `Repo` itself implements a generic interface. The
    /// concrete type args on `WidgetStore`'s own `extends` clause must
    /// resolve correctly regardless of what `Repo` does internally.
    #[test]
    fn two_hop_inheritance_resolves_instantiated_parent() {
        let (_asts, resolver) = resolve_fixture();
        let widget_store = resolved(
            &resolver,
            &["nres", "store"],
            &["nres", "store", "WidgetStore"],
        );
        let resolved_types::TypeKind::Class { inherit_class, .. } = &widget_store.type_kind else {
            panic!("expected a class");
        };
        let parent = inherit_class.as_ref().unwrap();
        assert_eq!(
            parent.name,
            fqn_in_project(&["nres", "generics"], &["nres", "generics", "Repo"])
        );
        assert_eq!(
            parent.type_arg_list.0[0],
            resolved_types::TypeArg::Is(resolved_types::RefType {
                name: fqn_in_project(&["nres", "store"], &["nres", "store", "Widget"]),
                type_arg_list: resolved_types::TypeArgList(vec![]),
                arr_dim: 0,
            })
        );
        assert_eq!(
            parent.type_arg_list.0[1],
            resolved_types::TypeArg::Is(resolved_types::RefType {
                name: resolved_types::FullyQualifiedName {
                    source: TypeSource::ExternalDependencyType,
                    typename: qn(&["String"]),
                },
                type_arg_list: resolved_types::TypeArgList(vec![]),
                arr_dim: 0,
            })
        );
    }

    /// Two interfaces that each export an unrelated nested type of the same
    /// relative name ("Shared") produce a diamond: implementing both marks
    /// the inherited name Ambiguous. Black-box replacement for the deleted
    /// `export_types.rs` unit test of the same shape -- driven through the
    /// real `Resolver` instead of hand-built `ExportedInnerTypes` fixtures.
    #[test]
    fn diamond_implements_produces_ambiguous_export() {
        let (_asts, resolver) = resolve_fixture();
        let entry = resolver
            .project
            .get_package(&qn(&["nres", "diamond"]))
            .unwrap()
            .get_type(&qn(&["nres", "diamond", "DiamondImpl"]))
            .unwrap();
        let exported = entry.export_types.as_ref().unwrap();
        let shared = &exported.0.get(&qn(&["Shared"])).unwrap().name;
        assert!(matches!(shared, ExportedTypeEntryName::Ambiguous));
    }

    /// A type's own nested type shadows an inherited one of the same name,
    /// rather than conflicting with it.
    #[test]
    fn own_nested_type_shadows_inherited_export() {
        let (_asts, resolver) = resolve_fixture();
        let entry = resolver
            .project
            .get_package(&qn(&["nres", "diamond"]))
            .unwrap()
            .get_type(&qn(&["nres", "diamond", "ShadowsShared"]))
            .unwrap();
        let exported = entry.export_types.as_ref().unwrap();
        let shared = &exported.0.get(&qn(&["Shared"])).unwrap().name;
        assert!(matches!(
            shared,
            ExportedTypeEntryName::Own(f) if *f == fqn_in_project(
                &["nres", "diamond"],
                &["nres", "diamond", "ShadowsShared", "Shared"]
            )
        ));
    }

    /// An interface extending another interface resolves its
    /// `extend_interfaces` list.
    #[test]
    fn interface_extending_interface_resolves() {
        let (_asts, resolver) = resolve_fixture();
        let pet = resolved(&resolver, &["nres", "animals"], &["nres", "animals", "Pet"]);
        let resolved_types::TypeKind::Interface { extend_interfaces } = &pet.type_kind else {
            panic!("expected an interface");
        };
        assert_eq!(
            extend_interfaces[0].name,
            fqn_in_project(&["nres", "animals"], &["nres", "animals", "Animal"])
        );
    }

    /// An enum implementing an interface resolves its `implement_interfaces`
    /// list, alongside its own enum constants.
    #[test]
    fn enum_implementing_interface_resolves() {
        let (_asts, resolver) = resolve_fixture();
        let priority = resolved(
            &resolver,
            &["nres", "status"],
            &["nres", "status", "Priority"],
        );
        let resolved_types::TypeKind::Enum {
            implement_interfaces,
            enum_vals,
        } = &priority.type_kind
        else {
            panic!("expected an enum");
        };
        assert_eq!(
            implement_interfaces[0].name,
            fqn_in_project(&["nres", "common"], &["nres", "common", "Describable"])
        );
        assert_eq!(
            enum_vals,
            &vec!["LOW".to_string(), "MEDIUM".to_string(), "HIGH".to_string()]
        );
    }

    /// A fully qualified, unimported type reference resolves via the
    /// project-index fallback tier once the scope lookup misses.
    #[test]
    fn fully_qualified_reference_without_import_resolves() {
        let (_asts, resolver) = resolve_fixture();
        let main = resolved(&resolver, &["nres", "app"], &["nres", "app", "Main"]);
        assert_eq!(
            property_type(member(main, "marker")),
            &fqn_in_project(&["nres", "diamond"], &["nres", "diamond", "DiamondImpl"])
        );
    }

    /// `construct_tree` reconstructs nested types bottom-up into
    /// `body.subtypes`, unlike the flat `resolved_node` used by the other
    /// tests here.
    #[test]
    fn nested_subtypes_reconstructed_bottom_up() {
        let (asts, mut resolver) = resolve_fixture();
        let ast = find_ast(&asts, &["nres", "wildcard"], "Gadget.java");
        let tree = resolver.construct_tree(ast);

        let gadget = &tree.0[0];
        assert_eq!(
            gadget.name,
            fqn_in_project(&["nres", "wildcard"], &["nres", "wildcard", "Gadget"])
        );

        let part = &gadget.body.subtypes[0];
        assert_eq!(
            part.name,
            fqn_in_project(
                &["nres", "wildcard"],
                &["nres", "wildcard", "Gadget", "Part"]
            )
        );

        let sub_part = &part.body.subtypes[0];
        assert_eq!(
            sub_part.name,
            fqn_in_project(
                &["nres", "wildcard"],
                &["nres", "wildcard", "Gadget", "Part", "SubPart"]
            )
        );
    }
}
