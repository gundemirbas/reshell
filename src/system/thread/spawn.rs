use crate::syscalls::{sys_clone_with_func, CLONE_VM, CLONE_FS, CLONE_FILES, CLONE_SIGHAND, CLONE_THREAD};
use super::thread_stack::ThreadStack;

/// Safe wrapper for spawning threads with custom stacks
/// 
/// This provides a safe interface over the unsafe `sys_clone_with_func` syscall.
/// The function pointer must be `fn() -> !` (diverging) because threads never return.
pub fn spawn_thread(stack: &ThreadStack, f: fn() -> !) -> Result<i32, &'static str> {
    let stack_top = stack.get_stack_top();
    if stack_top.is_null() {
        return Err("Invalid stack pointer");
    }

    let flags = CLONE_VM | CLONE_FS | CLONE_FILES | CLONE_SIGHAND | CLONE_THREAD;
    
    let tid = unsafe {
        sys_clone_with_func(flags, stack_top, f)
    };

    if tid < 0 {
        Err("Failed to clone thread")
    } else {
        Ok(tid as i32)
    }
}
