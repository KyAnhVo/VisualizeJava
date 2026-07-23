pub mod abstraction_graph;
pub mod name_resolution;
pub mod parser;
pub mod resolved_types;
pub mod types;

use std::{
    ffi::OsString,
    fs::{self, File},
    io::Write,
    path::PathBuf,
    rc::Rc,
};

use crate::{name_resolution::file_util::get_java_files, types::JavaFile};

#[derive(Debug, Clone, Copy)]
enum Flags {
    None,
    DebugAst,
    DebugFlattening,
    DebugNameResolution,
}

impl Flags {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "none" => Some(Self::None),
            "ast" => Some(Self::DebugAst),
            "flat" => Some(Self::DebugFlattening),
            "name-res" => Some(Self::DebugNameResolution),
            _ => None,
        }
    }
}

fn main() {
    use std::time::Instant;
    let start = Instant::now();

    // Start program

    let env_vars: Vec<OsString> = std::env::args_os().collect();

    let mut output_src: Box<dyn Write> = match env_vars.len() {
        3 => Box::new(std::io::stdout()),
        4 => Box::new(File::create(env_vars[2].clone().into_string().unwrap()).unwrap()),
        _ => panic!("Usage: <name> <src> [<output_file>] <mode>"),
    };
    let Ok(src) = env_vars[1].clone().into_string() else {
        panic!("requires UTF-8 dir");
    };
    let Some(flag) = Flags::from_str(
        env_vars[if env_vars.len() == 3 { 2 } else { 3 }]
            .clone()
            .into_string()
            .unwrap()
            .as_ref(),
    ) else {
        panic!("Mode: none, ast, flat, name-res");
    };

    let src_dir: PathBuf = src.into();
    let files = match get_java_files(src_dir.as_ref()) {
        Ok(f) => f,
        Err(e) => panic!("cannot detect file: {:#?}", e),
    };

    // Construct AST
    let mut asts: Vec<Rc<JavaFile>> = vec![];
    files.iter().for_each(|file| {
        let src_str = fs::read_to_string(file).unwrap();
        let ast = match parser::parser::Parser::parse(&src_str, file) {
            Ok(ast) => ast,
            Err(e) => panic!("Err: {:#?}", e),
        };
        asts.push(Rc::new(ast));
    });
    if let Flags::DebugAst = flag {
        write!(output_src, "{:#?}", &asts).unwrap();
        let duration = start.elapsed();
        println!("Time taken: {:?} microseconds", duration.as_micros());
        return;
    }

    // Construct type index
    let pkg_ind = name_resolution::resolve_types::PackageIndex::from_ast_lst(&asts);
    if let Flags::DebugFlattening = flag {
        write!(output_src, "{:#?}", pkg_ind).unwrap();
        let duration = start.elapsed();
        println!("Time taken: {:?} microseconds", duration.as_micros());
        return;
    }

    // End program
    let duration = start.elapsed();
    println!("Time taken: {:?} microseconds", duration.as_micros());
}
