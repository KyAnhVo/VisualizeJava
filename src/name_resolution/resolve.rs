use std::{collections::VecDeque, rc::Rc};

use crate::{
    name_resolution::{
        err::ReadProjectErr,
        resolve_types::{NameResolutionErr, PackageIndex},
        scope::Scope,
    },
    types::{self, QualifiedName},
};

pub struct TypeQueueEntry {
    pub name: QualifiedName,
    pub ast_root: Rc<types::JavaFile>,
    pub type_node: Rc<types::Type>,
    pub type_member_scope: Scope,
}
pub struct Resolver {
    pub queue: VecDeque<TypeQueueEntry>,
    pub project: PackageIndex,
    early_termination_counter: usize,
}

impl Resolver {
    pub fn new(asts: &[Rc<types::JavaFile>]) -> Result<Self, ReadProjectErr> {
        let project = PackageIndex::from_ast_lst(asts)?;
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

    pub fn deque_and_resolve(&mut self) -> Result<Self, NameResolutionErr> {
        if self.queue.len() == self.early_termination_counter {
            return Err(NameResolutionErr::CyclicDependency);
        }
        unimplemented!()
    }
}
