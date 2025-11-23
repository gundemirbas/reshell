use crate::syscalls::{write, STDOUT};
use crate::shell::parser::expand_env_vars;
use crate::io::print;

pub fn builtin_echo(args: &[u8]) {
    let mut expanded = [0u8; 512];
    let len = expand_env_vars(args, &mut expanded);
    write(STDOUT, &expanded[..len]);
    print(b"\n");
}
