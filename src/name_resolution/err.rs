use std::io;

use crate::types::ParseErr;

pub enum ReadProjectErr {
    IoErr(io::Error),
    ParseErr(ParseErr),
}

impl From<io::Error> for ReadProjectErr {
    fn from(e: io::Error) -> Self {
        ReadProjectErr::IoErr(e)
    }
}

impl From<ParseErr> for ReadProjectErr {
    fn from(e: ParseErr) -> Self {
        Self::ParseErr(e)
    }
}
