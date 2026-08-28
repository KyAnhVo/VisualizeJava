use crate::name_resolution::resolve_types::{Package, Project};
use crate::name_resolution::scope::Scope;
use crate::resolved_types::{FullyQualifiedName, TypeSource};
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
    pub fn from_type(node: Rc<types::Type>, scope: &Scope, project: &Project) -> Self {
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
                    let fqn = scope.resolve_qualified_name(name, project);
                    if let TypeSource::InProjectType { package } = fqn.source {
                        let pkg = project.get_package(&package).unwrap();
                        let type_ind = pkg.get_type(&fqn.typename).unwrap();
                        inheritance_import_vec.push(type_ind.export_types.as_ref().unwrap());
                    }
                }
                for s in implement_interfaces.iter() {
                    let name = &s.name;
                    let fqn = scope.resolve_qualified_name(name, project);
                    if let TypeSource::InProjectType { package } = fqn.source {
                        let pkg = project.get_package(&package).unwrap();
                        let type_ind = pkg.get_type(&fqn.typename).unwrap();
                        inheritance_import_vec.push(type_ind.export_types.as_ref().unwrap());
                    }
                }
            }
            Interface { extend_interfaces } => {
                for s in extend_interfaces.iter() {
                    let name = &s.name;
                    let fqn = scope.resolve_qualified_name(name, project);
                    if let TypeSource::InProjectType { package } = fqn.source {
                        let pkg = project.get_package(&package).unwrap();
                        let type_ind = pkg.get_type(&fqn.typename).unwrap();
                        inheritance_import_vec.push(type_ind.export_types.as_ref().unwrap());
                    }
                }
            }
            Enum {
                implement_interfaces,
                ..
            } => {
                for s in implement_interfaces.iter() {
                    let name = &s.name;
                    let fqn = scope.resolve_qualified_name(name, project);
                    if let TypeSource::InProjectType { package } = fqn.source {
                        let pkg = project.get_package(&package).unwrap();
                        let type_ind = pkg.get_type(&fqn.typename).unwrap();
                        inheritance_import_vec.push(type_ind.export_types.as_ref().unwrap());
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
