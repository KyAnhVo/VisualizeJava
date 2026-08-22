use crate::abstraction_graph::graph_types::{Edge, EdgeVariant, Node, TypeVariant};
use crate::resolved_types::*;
use crate::types::QualifiedName;
use std::collections::HashMap;
use std::rc::Rc;
#[derive(Debug)]
pub struct Graph(pub HashMap<QualifiedName, Node>);

impl Graph {
    pub fn from_trees(trees: &[FileTypeTree]) -> Self {
        let mut res = Self(HashMap::new());
        for tree in trees.iter() {
            res.build_from_tree_no_relationship(tree);
        }
        res
    }

    fn build_relationship(&mut self, tree: &FileTypeTree) {
        for typeclass in tree.0.iter().cloned() {
            self.build_inheritance_relationship_from_node(typeclass);
        }
    }

    fn build_inheritance_relationship_from_node(&mut self, node: Rc<Type>) {
        use EdgeVariant::*;

        // Build inheritance / implements edge
        match &node.type_kind {
            TypeKind::Class {
                inherit_class,
                implement_interfaces,
            } => {
                if let Some(parent) = inherit_class {
                    self.build_edge(&parent.name, &node.name, Extends);
                };
                for parent in implement_interfaces.iter() {
                    self.build_edge(&parent.name, &node.name, Implements);
                }
            }
            TypeKind::Enum {
                implement_interfaces,
                ..
            } => {
                for parent in implement_interfaces.iter() {
                    self.build_edge(&parent.name, &node.name, Implements);
                }
            }
            TypeKind::Interface { extend_interfaces } => {
                for parent in extend_interfaces.iter() {
                    self.build_edge(&parent.name, &node.name, Implements);
                }
            }
            TypeKind::Annotation { .. } => {}
        }
    }
    fn build_from_tree_no_relationship(&mut self, tree: &FileTypeTree) {
        for node in tree.0.iter() {
            self.build_node_no_relationship(node.clone());
        }
    }

    fn build_edge(
        &mut self,
        from: &FullyQualifiedName,
        to: &FullyQualifiedName,
        edge_variant: EdgeVariant,
    ) {
        self.0
            .get_mut(&from.typename)
            .unwrap()
            .out_edges
            .push(Edge {
                typename: to.clone(),
                variant: edge_variant,
            });
        self.0.get_mut(&to.typename).unwrap().in_edges.push(Edge {
            typename: from.clone(),
            variant: edge_variant,
        });
    }
    fn build_node_no_relationship(&mut self, tree_node: Rc<Type>) {
        self.0
            .entry(tree_node.name.typename.clone())
            .or_insert(Node {
                name: tree_node.name.clone(),
                type_variant: TypeVariant::from_typekind(&tree_node.type_kind),
                out_edges: vec![],
                in_edges: vec![],
            });
        for subtype in tree_node.body.subtypes.iter() {
            self.build_node_no_relationship(subtype.clone());
        }
        for member in tree_node.body.members.iter() {
            match &member.member_kind {
                MemberKind::Method { .. } => {
                    todo!("implement this");
                }
                MemberKind::Constructor { .. } => {
                    todo!("implement this");
                }
                MemberKind::Property { reftype, arr_dim } => {
                    todo!("implement this");
                }
            }
        }
    }
}
