use crate::io::{print, print_number};

pub fn builtin_threads() {
    use crate::system::thread::get_thread_stats;
    
    let (active, max) = get_thread_stats();
    
    print(b"Thread Statistics:\n");
    print(b"  Active threads: ");
    print_number(active as i64);
    print(b"\n  Maximum threads: ");
    print_number(max as i64);
    print(b"  Memory per thread: 128 KB (mmap-allocated)\n");
    print(b"  Total memory used: ");
    print_number((active * 128) as i64);
    print(b" KB\n");
}
