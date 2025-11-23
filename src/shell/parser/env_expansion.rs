use crate::shell::storage::ENV_STORAGE;

pub fn expand_env_vars(input: &[u8], output: &mut [u8]) -> usize {
    let mut out_idx = 0;
    let mut i = 0;
    
    while i < input.len() && input[i] != 0 {
        if input[i] == b'$' && i + 1 < input.len() {
            i += 1;
            let var_start = i;
            while i < input.len() && (input[i].is_ascii_alphanumeric() || input[i] == b'_') {
                i += 1;
            }
            
            if i > var_start {
                let var_name = &input[var_start..i];
                let remaining = &mut output[out_idx..];
                let len = ENV_STORAGE.get(var_name, remaining);
                out_idx += len;
            }
        } else {
            if out_idx >= output.len() { return out_idx; }
            output[out_idx] = input[i];
            out_idx += 1;
            i += 1;
        }
    }
    
    out_idx
}
