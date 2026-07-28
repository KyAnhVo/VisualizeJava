use crate::name_resolution::resolve_types::{Package, Project};
use crate::resolved_types::FullyQualifiedName;
use crate::types::{self, AccessModifier, QualifiedName};
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct ExportedTypeEntry {
    pub name: ExportedTypeEntryName,
    pub visibility: AccessModifier,
    pub root_package: Rc<QualifiedName>,
}

#[derive(Debug, Clone)]
pub enum ExportedTypeEntryName {
    Inherited(FullyQualifiedName),
    Own(FullyQualifiedName),
    Ambiguous,
}

/// ExportedInnerTypes is viewed by structs that will extend/implement it.
#[derive(Debug, Clone)]
pub struct ExportedInnerTypes(pub HashMap<QualifiedName, ExportedTypeEntry>);

impl ExportedInnerTypes {
    pub fn from_type(node: Rc<types::Type>, project: &Project) -> Self {
        use types::TypeKind::*;
        let (pkg, _) = project.get_origin_package(&node.name).unwrap();
        let mut res = Self::from_type_pre_inheritance(node.clone(), &pkg);
        let mut inheritance_import_vec: Vec<&Self> = vec![];
        match &node.type_kind {
            Class {
                inherit_class,
                implement_interfaces,
            } => {
                if let Some(s) = inherit_class {
                    let name = &s.name;
                    if let Some((_, ext_entry)) = project.get_origin_package(name) {
                        inheritance_import_vec.push(ext_entry.export_types.as_ref().unwrap());
                    }
                }
                for s in implement_interfaces.iter() {
                    let name = &s.name;
                    if let Some((_, ext_entry)) = project.get_origin_package(name) {
                        inheritance_import_vec.push(ext_entry.export_types.as_ref().unwrap());
                    }
                }
            }
            Interface { extend_interfaces } => {
                for s in extend_interfaces.iter() {
                    let name = &s.name;
                    if let Some((_, ext_entry)) = project.get_origin_package(name) {
                        inheritance_import_vec.push(ext_entry.export_types.as_ref().unwrap());
                    }
                }
            }
            Enum {
                implement_interfaces,
                ..
            } => {
                for s in implement_interfaces.iter() {
                    let name = &s.name;
                    if let Some((_, ext_entry)) = project.get_origin_package(name) {
                        inheritance_import_vec.push(ext_entry.export_types.as_ref().unwrap());
                    }
                }
            }
            Annotation { .. } => {}
        }
        res.import_inheritance(&inheritance_import_vec);
        res
    }

    pub fn from_type_pre_inheritance(node: Rc<types::Type>, package: &Package) -> Self {
        use ExportedTypeEntryName::*;
        // won't check if node actually has no parent.
        let prefix = &node.name;
        let mut res = Self(HashMap::new());
        let from_package = Rc::new(package.package.clone());

        for (name, entry) in package.iter() {
            if !name.has_proper_prefix(prefix) {
                continue;
            }
            let k = name.get_suffix(prefix.len()).unwrap();
            let v = ExportedTypeEntry {
                name: Own(package.get_fqn(name).unwrap()),
                visibility: entry.visibility,
                root_package: from_package.clone(),
            };
            res.0.insert(k, v);
        }

        res
    }

    pub fn import_inheritance(&mut self, inheritances: &[&Self]) {
        use ExportedTypeEntryName::*;

        for inheritance in inheritances.iter() {
            for (name, entry) in inheritance.0.iter() {
                if let Some(my_entry) = self.0.get(name) {
                    // if the name is in here already, then it might be
                    // from Own (shadow) or from another branch (implement/extend)
                    // of inheritance (ambiguous)
                    match &my_entry.name {
                        Own(_) => {}
                        Inherited(s) => match &entry.name {
                            Inherited(s1) | Own(s1) => {
                                if s != s1 {
                                    self.0.get_mut(name).unwrap().name = Ambiguous;
                                }
                            }
                            Ambiguous => {
                                self.0.get_mut(name).unwrap().name = Ambiguous;
                            }
                        },
                        Ambiguous => {
                            self.0.get_mut(name).unwrap().name = Ambiguous;
                        }
                    }
                    continue;
                } else {
                    self.0.insert(
                        name.clone(),
                        ExportedTypeEntry {
                            name: match &entry.name {
                                Own(s) | Inherited(s) => Inherited(s.clone()),
                                Ambiguous => Ambiguous,
                            },
                            visibility: entry.visibility,
                            root_package: entry.root_package.clone(),
                        },
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::name_resolution::resolve_types::test::load_project;
    use crate::resolved_types::TypeSource;

    fn qn(parts: &[&str]) -> QualifiedName {
        QualifiedName(parts.iter().map(|s| s.to_string()).collect())
    }

    /// Builds the expected FullyQualifiedName for an in-project type, where
    /// `suffix` is relative to `package`.
    fn fqn(package: &[&str], suffix: &[&str]) -> FullyQualifiedName {
        let mut typename: Vec<String> = package.iter().map(|s| s.to_string()).collect();
        typename.extend(suffix.iter().map(|s| s.to_string()));
        FullyQualifiedName {
            source: TypeSource::InProjectType {
                package: qn(package),
            },
            typename: QualifiedName(typename),
        }
    }

    fn type_node(project: &Project, pkg: &[&str], full_name: &[&str]) -> Rc<types::Type> {
        project
            .get_package(&qn(pkg))
            .unwrap()
            .get_type(&qn(full_name))
            .unwrap()
            .unresolved_node
            .clone()
    }

    /// Populates a type's `export_types`, standing in for what the (not yet
    /// implemented) resolver's BFS would do once a parent is visited --
    /// `from_type` expects parents to already have this filled in.
    fn resolve_export_types(project: &mut Project, pkg: &[&str], full_name: &[&str]) {
        let pkg_key = qn(pkg);
        let type_key = qn(full_name);
        let node = type_node(project, pkg, full_name);
        let exported = {
            let package = project.get_package(&pkg_key).unwrap();
            ExportedInnerTypes::from_type_pre_inheritance(node, package)
        };
        project
            .get_mut_package(&pkg_key)
            .unwrap()
            .get_mut_type(&type_key)
            .unwrap()
            .export_types = Some(exported);
    }

    fn name_of<'a>(exported: &'a ExportedInnerTypes, key: &str) -> &'a ExportedTypeEntryName {
        &exported.0.get(&qn(&[key])).unwrap().name
    }

    // ------------------------- from_type_pre_inheritance -------------------------

    /// The pre-inheritance export map lists every nested descendant of a
    /// type, keyed relative to the type itself and tagged with each entry's
    /// own visibility -- including private ones. Visibility filtering isn't
    /// this function's job (compare to `Scope`'s `add_*` tiers, which do
    /// filter); it happens in whatever later consumes this raw map.
    #[test]
    fn pre_inheritance_lists_all_nested_descendants_regardless_of_visibility() {
        let (_asts, project) = load_project("test_target_scope");
        let pkg = project.get_package(&qn(&["scope", "pkg"])).unwrap();
        let outer = type_node(&project, &["scope", "pkg"], &["scope", "pkg", "Outer"]);

        let exported = ExportedInnerTypes::from_type_pre_inheritance(outer, pkg);

        let inner = exported.0.get(&qn(&["Inner"])).unwrap();
        assert!(
            matches!(&inner.name, ExportedTypeEntryName::Own(f) if *f == fqn(&["scope", "pkg"], &["Outer", "Inner"]))
        );
        assert_eq!(inner.visibility, AccessModifier::Public);

        let deep = exported.0.get(&qn(&["Inner", "Deep"])).unwrap();
        assert!(
            matches!(&deep.name, ExportedTypeEntryName::Own(f) if *f == fqn(&["scope", "pkg"], &["Outer", "Inner", "Deep"]))
        );

        let hidden = exported.0.get(&qn(&["Hidden"])).unwrap();
        assert_eq!(hidden.visibility, AccessModifier::Private);

        // the type itself is not counted as its own descendant.
        assert!(exported.0.get(&qn(&["Outer"])).is_none());
    }

    // ---------------------------- import_inheritance ------------------------------

    fn dummy_root() -> Rc<QualifiedName> {
        Rc::new(qn(&["dummy"]))
    }

    fn id(label: &str) -> FullyQualifiedName {
        FullyQualifiedName {
            source: TypeSource::Generic,
            typename: qn(&[label]),
        }
    }

    fn single_entry(key: &str, name: ExportedTypeEntryName) -> ExportedInnerTypes {
        let mut m = HashMap::new();
        m.insert(
            qn(&[key]),
            ExportedTypeEntry {
                name,
                visibility: AccessModifier::Public,
                root_package: dummy_root(),
            },
        );
        ExportedInnerTypes(m)
    }

    /// An owned entry is never overwritten by an inherited one of the same
    /// name, even if the inherited side is itself ambiguous.
    #[test]
    fn import_inheritance_own_beats_inherited_and_ambiguous() {
        let mut own = single_entry("Shared", ExportedTypeEntryName::Own(id("self")));
        let inherited = single_entry("Shared", ExportedTypeEntryName::Inherited(id("parent")));
        let ambiguous = single_entry("Shared", ExportedTypeEntryName::Ambiguous);

        own.import_inheritance(&[&inherited, &ambiguous]);

        assert!(
            matches!(name_of(&own, "Shared"), ExportedTypeEntryName::Own(f) if *f == id("self"))
        );
    }

    /// A name with no existing entry is adopted as `Inherited`, whether the
    /// source entry was itself `Own` or already `Inherited`.
    #[test]
    fn import_inheritance_new_name_is_adopted_as_inherited() {
        let mut own = ExportedInnerTypes(HashMap::new());
        let parent = single_entry("Shared", ExportedTypeEntryName::Own(id("parent")));

        own.import_inheritance(&[&parent]);

        assert!(
            matches!(name_of(&own, "Shared"), ExportedTypeEntryName::Inherited(f) if *f == id("parent"))
        );
    }

    /// The same ancestor reached through two different inheritance paths
    /// (e.g. two interfaces re-exporting a common grandparent's nested type)
    /// is not ambiguous -- only genuinely different origins are.
    #[test]
    fn import_inheritance_same_origin_via_two_paths_is_not_ambiguous() {
        let mut own = single_entry("Shared", ExportedTypeEntryName::Inherited(id("common")));
        let via_other_path = single_entry("Shared", ExportedTypeEntryName::Inherited(id("common")));

        own.import_inheritance(&[&via_other_path]);

        assert!(
            matches!(name_of(&own, "Shared"), ExportedTypeEntryName::Inherited(f) if *f == id("common"))
        );
    }

    /// Two different inherited origins for the same relative name is the
    /// diamond case: it must be marked `Ambiguous`.
    #[test]
    fn import_inheritance_conflicting_origins_become_ambiguous() {
        let mut own = single_entry("Shared", ExportedTypeEntryName::Inherited(id("via_a")));
        let via_b = single_entry("Shared", ExportedTypeEntryName::Inherited(id("via_b")));

        own.import_inheritance(&[&via_b]);

        assert!(matches!(
            name_of(&own, "Shared"),
            ExportedTypeEntryName::Ambiguous
        ));
    }

    /// Ambiguity is contagious: merging in an already-ambiguous entry makes
    /// the (non-Own) result ambiguous too, and an already-ambiguous entry
    /// stays that way regardless of what merges in afterward.
    #[test]
    fn import_inheritance_ambiguity_is_contagious_and_sticky() {
        let mut inherited = single_entry("Shared", ExportedTypeEntryName::Inherited(id("via_a")));
        let incoming_ambiguous = single_entry("Shared", ExportedTypeEntryName::Ambiguous);
        inherited.import_inheritance(&[&incoming_ambiguous]);
        assert!(matches!(
            name_of(&inherited, "Shared"),
            ExportedTypeEntryName::Ambiguous
        ));

        let mut already_ambiguous = single_entry("Shared", ExportedTypeEntryName::Ambiguous);
        let unrelated = single_entry("Shared", ExportedTypeEntryName::Inherited(id("via_b")));
        already_ambiguous.import_inheritance(&[&unrelated]);
        assert!(matches!(
            name_of(&already_ambiguous, "Shared"),
            ExportedTypeEntryName::Ambiguous
        ));
    }

    // -------------------------------- from_type ------------------------------------
    //
    // Fixture note: `test_target_export/diamond` deliberately uses fully
    // qualified names in `implements` clauses (`implements
    // export.diamond.IfaceA`), not the bare `IfaceA` real-world Java would
    // use. `from_type` resolves parent references via
    // `Project::get_origin_package`, which only searches package-prefixed
    // (already-qualified) names -- it does not consult `Scope`. A bare
    // `IfaceA` would make `get_origin_package` return `None` (silently, no
    // error) and `from_type` would see zero inherited members. That's a gap
    // for whoever wires `from_type` into the resolver later: parent RefType
    // names need to go through `Scope::resolve_qualified_name` first.

    /// A type implementing a single interface inherits that interface's
    /// exported nested types unmodified.
    #[test]
    fn from_type_inherits_cleanly_through_one_interface() {
        let (_asts, mut project) = load_project("test_target_export");
        resolve_export_types(
            &mut project,
            &["export", "diamond"],
            &["export", "diamond", "IfaceA"],
        );

        let only_a = type_node(
            &project,
            &["export", "diamond"],
            &["export", "diamond", "OnlyA"],
        );
        let exported = ExportedInnerTypes::from_type(only_a, &project);

        assert!(matches!(
            name_of(&exported, "Shared"),
            ExportedTypeEntryName::Inherited(f) if *f == fqn(&["export", "diamond"], &["IfaceA", "Shared"])
        ));
    }

    /// A type's own nested type shadows an inherited one of the same name,
    /// same as a subclass's own member hiding an inherited one in Java.
    #[test]
    fn from_type_own_nested_type_shadows_inherited() {
        let (_asts, mut project) = load_project("test_target_export");
        resolve_export_types(
            &mut project,
            &["export", "diamond"],
            &["export", "diamond", "IfaceA"],
        );

        let shadows_a = type_node(
            &project,
            &["export", "diamond"],
            &["export", "diamond", "ShadowsA"],
        );
        let exported = ExportedInnerTypes::from_type(shadows_a, &project);

        assert!(matches!(
            name_of(&exported, "Shared"),
            ExportedTypeEntryName::Own(f) if *f == fqn(&["export", "diamond"], &["ShadowsA", "Shared"])
        ));
    }

    /// Implementing two interfaces that each export an unrelated nested type
    /// of the same relative name produces a diamond: `Shared` is ambiguous,
    /// while the implementing type's own unrelated nested type is untouched.
    #[test]
    fn from_type_diamond_from_two_interfaces_is_ambiguous() {
        let (_asts, mut project) = load_project("test_target_export");
        resolve_export_types(
            &mut project,
            &["export", "diamond"],
            &["export", "diamond", "IfaceA"],
        );
        resolve_export_types(
            &mut project,
            &["export", "diamond"],
            &["export", "diamond", "IfaceB"],
        );

        let impl_node = type_node(
            &project,
            &["export", "diamond"],
            &["export", "diamond", "Impl"],
        );
        let exported = ExportedInnerTypes::from_type(impl_node, &project);

        assert!(matches!(
            name_of(&exported, "Shared"),
            ExportedTypeEntryName::Ambiguous
        ));
        assert!(matches!(
            name_of(&exported, "Own"),
            ExportedTypeEntryName::Own(f) if *f == fqn(&["export", "diamond"], &["Impl", "Own"])
        ));
    }
}
