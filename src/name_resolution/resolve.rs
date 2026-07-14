use std::rc::Rc;

use crate::{
    name_resolution::resolve_types::{FlattenProject, Scope},
    types::{JavaFile, Member},
};

pub fn resolve_ast(ast: &mut JavaFile, flatten_project: &FlattenProject) {}

fn resolve_member(member: &mut Member, scope: Scope) {}
