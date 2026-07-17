use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// implements a stack. Purely for semantic reasons.
#[derive(Debug)]
pub struct Stack<T> {
    item: Vec<T>,
}
impl<T> Stack<T> {
    pub fn new() -> Self {
        Self { item: Vec::new() }
    }

    pub fn push(&mut self, obj: T) {
        self.item.push(obj);
    }

    pub fn pop(&mut self) -> Option<T> {
        self.item.pop()
    }

    pub fn peek(&self) -> Option<&T> {
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
pub(crate) fn get_java_files(dir: &Path) -> io::Result<Vec<PathBuf>> {
    if !dir.is_dir() {
        if dir.extension().is_some_and(|x| x.eq("java")) {
            return Ok(vec![dir.to_path_buf()]);
        } else {
            return Ok(vec![]);
        }
    }

    let mut v: Vec<PathBuf> = vec![];
    let entries = fs::read_dir(dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        v.append(&mut get_java_files(&path)?);
    }

    Ok(v)
}
