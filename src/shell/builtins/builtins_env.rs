use crate::syscalls::{write, STDOUT};
use crate::shell::storage::ENV_STORAGE;
use crate::utils::trim_spaces;
use crate::io::print;

pub fn builtin_export(args: &[u8]) {
    let args = trim_spaces(args);
    
    if args.is_empty() {
        ENV_STORAGE.iter(|var| {
            write(STDOUT, var);
            print(b"\n");
        });
        return;
    }
    
    for i in 0..args.len() {
        if args[i] == b'=' {
            let name = &args[..i];
            let value = if i + 1 < args.len() {
                &args[i + 1..]
            } else {
                &[]
            };
            if !ENV_STORAGE.set(name, value) {
                print(b"export: too many variables\n");
            }
            return;
        }
    }
    
    print(b"export: invalid format (use NAME=VALUE)\n");
}
