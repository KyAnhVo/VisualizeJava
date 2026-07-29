use std::{collections::VecDeque, rc::Rc};

use crate::{
    name_resolution::{
        err::ReadProjectErr,
        export_types::ExportedInnerTypes,
        resolve_types::{NameResolutionErr, Project},
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

    pub fn resolve(asts: &[Rc<types::JavaFile>]) -> Rc<[Rc<resolved_types::FileTypeTree>]> {
        let resolved_trees: Vec<Rc<resolved_types::FileTypeTree>> = vec![];
        // 1. create a new Resolver.
        let mut resolver = Self::new(asts).unwrap();
        // 2. continuously dequeue and resolve
        while resolver.deque_and_resolve().unwrap() == ResolveStatus::Unfinished {}
        // 3. construct the trees from the ground up.
        todo!();

        resolved_trees.into()
    }

    pub fn construct_tree(&self, type_node: Rc<types::Type>) -> Rc<resolved_types::Type> {
        let subtypes: Vec<Rc<resolved_types::Type>> = type_node
            .body
            .subtypes
            .iter()
            .map(|subtype| self.construct_tree(subtype.clone()))
            .collect();
        todo!()
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
