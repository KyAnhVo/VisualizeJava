use crate::types::{JavaFile, QualifiedName};
use std::collections::HashMap;

pub(crate) struct Package {
    pub name: QualifiedName,
    pub files: Vec<JavaFile>,
}

pub(crate) struct Project(HashMap<QualifiedName, Package>);
