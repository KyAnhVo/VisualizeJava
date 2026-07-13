use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// implements a stack. Purely for semantic reasons.
#[derive(Debug)]
pub(crate) struct Stack<T> {
    item: Vec<T>,
}
impl<T> Stack<T> {
    pub(crate) fn new() -> Self {
        Self { item: Vec::new() }
    }

    pub(crate) fn push(&mut self, obj: T) {
        self.item.push(obj);
    }

    pub(crate) fn pop(&mut self) -> Option<T> {
        self.item.pop()
    }

    pub(crate) fn peek(&self) -> Option<&T> {
        self.item.last()
    }

    pub fn is_empty(&self) -> bool {
        self.item.is_empty()
    }
}
impl<T> Default for Stack<T> {
    fn default() -> Self {
        Self::new()
    }
}
pub(crate) fn get_java_files_recursive(
    dir: &Path,
    root_dir: &Path,
) -> io::Result<Vec<(PathBuf, String)>> {
    if !dir.is_dir() {
        if dir.extension().is_some_and(|x| x.eq("java")) {
            return Ok(vec![(
                dir.to_path_buf(),
                dir.strip_prefix(root_dir)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
            )]);
        } else {
            return Ok(vec![]);
        }
    }

    let mut v: Vec<(PathBuf, String)> = vec![];
    let entries = fs::read_dir(dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        v.append(&mut get_java_files_recursive(&path, root_dir)?);
    }

    Ok(v)
}
