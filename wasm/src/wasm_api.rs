use std::{path::Path, rc::Rc};

use wasm_bindgen::prelude::*;

use crate::{
    abstraction_graph::graph::Graph, name_resolution::resolve::Resolver, parser::parser::Parser,
    types::JavaFile,
};

/// Accumulates parsed Java files and builds an abstraction graph from them.
///
/// Each instance owns its own set of ASTs, so two builders never see each other's
/// files. Callers should `free()` an instance when done, or `clear()` it to reuse
/// it for another project.
#[wasm_bindgen]
pub struct ProjectBuilder {
    asts: Vec<Rc<JavaFile>>,
}

impl Default for ProjectBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl ProjectBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self { asts: Vec::new() }
    }

    /// Parses one file into the builder.
    ///
    /// Paths that are not `.java`, or that name `module-info.java` /
    /// `package-info.java`, are skipped silently. Invalid UTF-8 and parse failures
    /// return an error rather than panicking, so one bad file does not tear down
    /// the module.
    pub fn add_file(&mut self, path: String, data: &[u8]) -> Result<(), JsValue> {
        if !path.ends_with(".java")
            || path.ends_with("module-info.java")
            || path.ends_with("package-info.java")
        {
            return Ok(());
        }

        let src = std::str::from_utf8(data)
            .map_err(|e| JsValue::from_str(&format!("{path}: not valid UTF-8: {e}")))?;

        let ast = Parser::parse(src, &Path::new(&path).to_path_buf())
            .map_err(|e| JsValue::from_str(&format!("{path}: parse error: {e:?}")))?;

        self.asts.push(Rc::new(ast));
        Ok(())
    }

    /// Resolves names across every file added so far and returns the abstraction
    /// graph. Does not consume the builder — callers may add more files and rebuild.
    pub fn build_graph(&self) -> Result<JsValue, JsValue> {
        let resolved_tree = Resolver::resolve(&self.asts);
        let graph = Graph::from_trees(&resolved_tree);
        serde_wasm_bindgen::to_value(&graph).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Drops every file added so far, leaving the builder reusable.
    pub fn clear(&mut self) {
        self.asts.clear();
    }

    #[wasm_bindgen(getter)]
    pub fn file_count(&self) -> usize {
        self.asts.len()
    }
}
