use crate::name_resolution::file_util::Stack;
use crate::resolved_types::{self, FullyQualifiedName, PrimitiveType, TypeSource};
use crate::types::{self, QualifiedName};
use std::collections::HashMap;
use std::rc::Rc;

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
}

// ---------------------- Resolve here --------------------------

impl Scope {
    // ------------------------- Resolving members and types ------------------------

    fn resolve_member(&mut self, member: &types::Member) -> resolved_types::Member {
        resolved_types::Member {
            name: member.name.clone(),
            annotations: self.resolve_annotations(&member.annotations),
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
                } => unimplemented!(),
                types::MemberKind::Constructor {
                    type_param_list,
                    input,
                    throws,
                } => unimplemented!(),
            },
        }
    }

    fn resolve_annotations(
        &mut self,
        annotations: &Vec<Rc<types::Annotation>>,
    ) -> Vec<resolved_types::Annotation> {
        unimplemented!()
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
        let mut res: resolved_types::TypeArgList = resolved_types::TypeArgList(vec![]);

        typearg_list.0.iter().for_each(|typearg| {
            res.0.push(self.resolve_type_arg(typearg));
        });

        res
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
