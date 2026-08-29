use serde::Serialize;

use crate::abstraction_graph::graph_types::{Edge, EdgeVariant, Node, TypeVariant};
use crate::resolved_types::*;
use crate::types::QualifiedName;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug, Serialize)]
pub struct Graph(pub HashMap<QualifiedName, Node>);

impl Graph {
    pub fn from_trees(trees: &[Rc<FileTypeTree>]) -> Self {
        let mut res = Self(HashMap::new());
        for tree in trees.iter() {
            res.build_from_tree_no_relationship(tree);
        }
        for tree in trees.iter() {
            res.build_relationships_from_tree(tree);
        }
        res
    }

    fn build_relationships_from_tree(&mut self, tree: &FileTypeTree) {
        for node in tree.0.iter() {
            self.build_relationships_from_node(node.clone());
        }
    }

    fn build_relationships_from_node(&mut self, tree_node: Rc<Type>) {
        self.build_inheritance_relationship_from_node(tree_node.clone());
        self.build_associative_relationshup_from_node(tree_node.clone());
        for subtype in tree_node.body.subtypes.iter() {
            self.build_relationships_from_node(subtype.clone());
        }
    }

    fn build_associative_relationshup_from_node(&mut self, tree_node: Rc<Type>) {
        use EdgeVariant::Association;
        for member in tree_node.body.members.iter() {
            match &member.member_kind {
                MemberKind::Method {
                    type_param_list,
                    input,
                    output,
                    throws,
                } => {
                    for typeclass in type_param_list.0.iter() {
                        self.build_edge(
                            &tree_node.name,
                            &typeclass.name,
                            Association(member.clone()),
                        );
                    }
                    for typeclass in input.iter() {
                        self.build_edge(
                            &tree_node.name,
                            &typeclass.name,
                            Association(member.clone()),
                        );
                    }
                    if let VoidableType::RefType(reftype) = output {
                        self.build_edge(
                            &tree_node.name,
                            &reftype.name,
                            Association(member.clone()),
                        );
                    }
                    for typeclass in throws.iter() {
                        self.build_edge(
                            &tree_node.name,
                            &typeclass.name,
                            Association(member.clone()),
                        );
                    }
                }
                MemberKind::Constructor {
                    type_param_list,
                    input,
                    throws,
                } => {
                    for typeclass in type_param_list.0.iter() {
                        self.build_edge(
                            &tree_node.name,
                            &typeclass.name,
                            Association(member.clone()),
                        );
                    }
                    for typeclass in input.iter() {
                        self.build_edge(
                            &tree_node.name,
                            &typeclass.name,
                            Association(member.clone()),
                        );
                    }
                    for typeclass in throws.iter() {
                        self.build_edge(
                            &tree_node.name,
                            &typeclass.name,
                            Association(member.clone()),
                        );
                    }
                }
                MemberKind::Property { reftype, .. } => {
                    self.build_edge(&tree_node.name, &reftype.name, Association(member.clone()));
                }
            }
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
                    self.build_edge(&node.name, &parent.name, Extends);
                };
                for parent in implement_interfaces.iter() {
                    self.build_edge(&node.name, &parent.name, Implements);
                }
            }
            TypeKind::Enum {
                implement_interfaces,
                ..
            } => {
                for parent in implement_interfaces.iter() {
                    self.build_edge(&node.name, &parent.name, Implements);
                }
            }
            TypeKind::Interface { extend_interfaces } => {
                for parent in extend_interfaces.iter() {
                    self.build_edge(&node.name, &parent.name, Implements);
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

    fn build_assoc_edge_from_reftype(
        &mut self,
        from: &FullyQualifiedName,
        to: &FullyQualifiedName,
        edge_variant: EdgeVariant,
        reftype: RefType,
    ) {
    }

    fn build_edge(
        &mut self,
        from: &FullyQualifiedName,
        to: &FullyQualifiedName,
        edge_variant: EdgeVariant,
    ) {
        let (Some(from_key), Some(to_key)) = (from.into_fqn(), to.into_fqn()) else {
            return;
        };
        if !self.0.contains_key(&from_key) || !self.0.contains_key(&to_key) {
            return;
        }
        self.0.get_mut(&from_key).unwrap().out_edges.push(Edge {
            typename: to.clone(),
            variant: edge_variant.clone(),
        });
        self.0.get_mut(&to_key).unwrap().in_edges.push(Edge {
            typename: from.clone(),
            variant: edge_variant.clone(),
        });
    }
    fn build_node_no_relationship(&mut self, tree_node: Rc<Type>) {
        let key = tree_node
            .name
            .into_fqn()
            .expect("declared project type must have a resolvable fully-qualified name");
        self.0.entry(key).or_insert(Node {
            name: tree_node.name.clone(),
            type_variant: TypeVariant::from_typekind(&tree_node.type_kind),
            members: tree_node.body.members.clone(),
            out_edges: vec![],
            in_edges: vec![],
        });
        for subtype in tree_node.body.subtypes.iter() {
            self.build_node_no_relationship(subtype.clone());
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::name_resolution::resolve::Resolver;
    use crate::name_resolution::resolve_types::test::load_project;

    fn build_graph(dir: &str) -> Graph {
        let (asts, _project) = load_project(dir);
        let trees = Resolver::resolve(&asts);
        let trees: Rc<[Rc<FileTypeTree>]> = trees.iter().map(|t| (t).clone()).collect();
        Graph::from_trees(&trees)
    }

    fn node_named<'a>(graph: &'a Graph, name: &str) -> &'a Node {
        graph
            .0
            .iter()
            .find(|(k, _)| k.0.last().map(String::as_str) == Some(name))
            .unwrap_or_else(|| panic!("{name} node should exist"))
            .1
    }

    /// End-to-end smoke test over a real (small) project: must not panic and
    /// must produce a node per declared type.
    ///
    /// `Book implements Comparable<Book>` must NOT produce an edge, since
    /// `Comparable` is outside the project and never gets a `Node` -- the
    /// graph only represents project-internal relationships.
    ///
    /// `BookRepository extends AbstractRepository<Book, String>` is
    /// project-internal and must produce a real `Extends` edge.
    #[test]
    fn builds_graph_for_small_project_without_panicking() {
        let graph = build_graph("test_target/small");

        let book = node_named(&graph, "Book");
        assert!(
            !book
                .out_edges
                .iter()
                .any(|e| e.typename.typename.0.last().map(String::as_str) == Some("Comparable")),
            "edges to types outside the project should be dropped"
        );

        let book_repository = node_named(&graph, "BookRepository");
        assert!(book_repository.out_edges.iter().any(|e| {
            e.variant == EdgeVariant::Extends
                && e.typename.typename.0.last().map(String::as_str) == Some("AbstractRepository")
        }));
    }

    /// `Loan` (`test_target/small/src/library/model/Loan.java`) has
    /// `Property` fields `book: Book` and `member: Member`, both
    /// project-internal types. Each should produce an `Association` edge,
    /// out of `Loan` and into the referenced type, tagged with the `Member`
    /// (field) that produced it.
    #[test]
    fn builds_association_edges_from_members() {
        let graph = build_graph("test_target/small");

        let loan = node_named(&graph, "Loan");

        assert!(
            loan.out_edges.iter().any(|e| {
                e.typename.typename.0.last().map(String::as_str) == Some("Book")
                    && matches!(&e.variant, EdgeVariant::Association(m) if m.name == "book")
            }),
            "Loan should have an Association edge to Book via its `book` field"
        );
        assert!(
            loan.out_edges.iter().any(|e| {
                e.typename.typename.0.last().map(String::as_str) == Some("Member")
                    && matches!(&e.variant, EdgeVariant::Association(m) if m.name == "member")
            }),
            "Loan should have an Association edge to Member via its `member` field"
        );

        // build_edge is symmetric: the referenced type should record the
        // same edge on its in_edges side.
        let book = node_named(&graph, "Book");
        assert!(
            book.in_edges.iter().any(|e| {
                e.typename.typename.0.last().map(String::as_str) == Some("Loan")
                    && matches!(&e.variant, EdgeVariant::Association(m) if m.name == "book")
            }),
            "Book should have the reciprocal Association edge from Loan on in_edges"
        );
    }
}
