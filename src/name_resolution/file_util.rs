use std::fs;
use std::io;
use std::path::{Path, PathBuf};

fn get_java_files_recursive(dir: &Path, root_dir: &Path) -> io::Result<Vec<(PathBuf, String)>> {
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
