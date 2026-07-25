use std::{
    collections::{HashMap, VecDeque},
    rc::Rc,
};

use crate::{
    name_resolution::{
        err::ReadProjectErr,
        resolve_types::{NameResolutionErr, Project},
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
            // TODO: implement resolution
            unimplemented!()
        } else {
            self.early_termination_counter += 1;
            self.queue.push_back(entry);
        }

        Ok(ResolveStatus::Unfinished)
    }

    fn resolve_entry(&mut self, entry: &TypeQueueEntry) {}

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
        let resolved_reftype = entry.type_member_scope.resolve_reftype(&reftype, project);
        let fqn = resolved_reftype.name;
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

pub struct InnerTypeScope(pub HashMap<QualifiedName, (FullyQualifiedName, AccessModifier)>);
