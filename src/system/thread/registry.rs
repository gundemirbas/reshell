use core::sync::atomic::{AtomicUsize, AtomicI32, Ordering};

const MAX_THREADS: usize = 256;

static THREAD_COUNT: AtomicUsize = AtomicUsize::new(0);
static THREAD_IDS: [AtomicI32; MAX_THREADS] = [const { AtomicI32::new(0) }; MAX_THREADS];

pub fn register_thread(tid: i32) -> bool {
    let count = THREAD_COUNT.load(Ordering::Acquire);
    if count >= MAX_THREADS {
        return false;
    }
    
    THREAD_IDS[count].store(tid, Ordering::Release);
    THREAD_COUNT.fetch_add(1, Ordering::Release);
    true
}

pub fn get_thread_stats() -> (usize, usize) {
    let count = THREAD_COUNT.load(Ordering::Acquire);
    (count, MAX_THREADS)
}

pub fn cleanup_threads() {
    let count = THREAD_COUNT.load(Ordering::Acquire);
    if count == 0 {
        return;
    }
    
    use crate::io::print;
    use crate::syscalls::{getpid, tgkill};
    
    print(b"[INFO] Cleaning up threads...\n");
    
    let tgid = getpid();
    
    for i in 0..count {
        let tid = THREAD_IDS[i].load(Ordering::Acquire);
        if tid > 0 {
            // Kill thread with SIGKILL (9)
            tgkill(tgid, tid, 9);
            THREAD_IDS[i].store(0, Ordering::Release);
        }
    }
    
    THREAD_COUNT.store(0, Ordering::Release);
    print(b"[INFO] Threads cleanup complete\n");
}
