use std::{io, path::PathBuf};

use crate::types::ParseErr;

#[derive(Debug)]
pub enum ReadProjectErr {
    IoErr(io::Error),
    ParseErr(ParseErr, PathBuf),
    SemanticErr(&'static str),
}

impl From<io::Error> for ReadProjectErr {
    fn from(e: io::Error) -> Self {
        ReadProjectErr::IoErr(e)
    }
}
