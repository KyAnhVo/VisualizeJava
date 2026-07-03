use std::collections::HashMap;

use crate::types::{QualifiedName, Type};

/// A Project is an wrapper for a hashmap
/// ```
/// f: QualifiedName -> Vec<Type>
/// ```
/// where semantically it maps a package name to all of its declared types.
pub struct Project<'a>(HashMap<QualifiedName<'a>, Vec<Type<'a>>>);

impl<'a> Project<'a> {}
