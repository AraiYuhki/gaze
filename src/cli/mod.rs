mod executor;
mod parser;

pub use executor::GitCli;
pub use parser::{parse_log, parse_stash_list, parse_status};
