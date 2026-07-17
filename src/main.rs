pub mod abstraction_graph;
pub mod name_resolution;
pub mod parser;
pub mod resolved_types;
pub mod types;

use std::{ffi::OsString, fs, path::PathBuf};

use crate::{name_resolution::file_util::get_java_files, types::JavaFile};

fn main() {
    let env_vars: Vec<OsString> = std::env::args_os().collect();
    assert!(env_vars.len() == 2, "Usage: <name> <src>");
    let Ok(src) = env_vars[1].clone().into_string() else {
        panic!("requires UTF-8 dir");
    };

    let src_dir: PathBuf = src.into();
    let files = match get_java_files(src_dir.as_ref()) {
        Ok(f) => f,
        Err(e) => panic!("cannot detect file: {:#?}", e),
    };
    let mut asts: Vec<JavaFile> = vec![];
    files.iter().for_each(|file| {
        let src_str = fs::read_to_string(file).unwrap();
        let ast = match parser::parser::Parser::parse(&src_str, file) {
            Ok(ast) => ast,
            Err(e) => panic!("Err: {:#?}", e),
        };
        asts.push(ast);
    });

    println!("{:#?}", asts);
}
