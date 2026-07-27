use std::{
    collections::{HashMap, VecDeque},
    rc::Rc,
};

use crate::{
    name_resolution::{
        err::ReadProjectErr,
        resolve_types::{NameResolutionErr, Package, Project},
        scope::Scope,
    },
    resolved_types::{FullyQualifiedName, TypeSource},
    types::{self, AccessModifier, QualifiedName},
};

#[derive(Debug, Clone)]
pub struct TypeQueueEntry {
    pub name: QualifiedName,
    pub ast_root: Rc<types::JavaFile>,
    pub type_node: Rc<types::Type>,
    pub type_member_scope: Scope,
}

#[derive(Debug, Clone, Copy)]
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
        let project = Project::from_ast_lst(asts)?;
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
            self.resolve_entry(&entry);
            unimplemented!()
        } else {
            self.early_termination_counter += 1;
            self.queue.push_back(entry);
        }

        Ok(ResolveStatus::Unfinished)
    }

    fn resolve_entry(&mut self, entry: &TypeQueueEntry) {
        let mut scope = &entry.type_member_scope;
        let package = self
            .project
            .get_package(&entry.ast_root.package_name)
            .unwrap();
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
        let fqn = entry
            .type_member_scope
            .resolve_qualified_name(&reftype.name, project);
        match fqn.source {
            TypeSource::InProjectType { package } => self.name_is_resolved(&fqn.typename, &package),
            _ => true,
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
