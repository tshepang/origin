#[cfg(feature = "origin-thread")]
use {
    alloc::boxed::Box,
    core::any::Any,
    core::arch::asm,
    core::ffi::c_void,
    linux_raw_sys::general::{__NR_clone, __NR_exit, __NR_munmap},
    rustix::thread::RawPid,
};

/// A wrapper around the Linux `clone` system call.
#[cfg(feature = "origin-thread")]
#[inline]
pub(super) unsafe fn clone(
    flags: u32,
    child_stack: *mut c_void,
    parent_tid: *mut RawPid,
    newtls: *mut c_void,
    child_tid: *mut RawPid,
    fn_: *mut Box<dyn FnOnce() -> Option<Box<dyn Any>>>,
) -> isize {
    let r0;
    asm!(
        "ecall",              // do the `clone` system call
        "bnez a0,0f",         // branch if we're in the parent thread

        // Child thread.
        "mv a0,{fn_}",        // pass `fn_` as the first argument
        "mv fp, zero",        // zero the frame address
        "mv ra, zero",        // zero the return address
        "tail {entry}",

        // Parent thread.
        "0:",

        entry = sym super::thread::entry,
        fn_ = in(reg) fn_,
        in("a7") __NR_clone,
        inlateout("a0") flags as isize => r0,
        in("a1") child_stack,
        in("a2") parent_tid,
        in("a3") newtls,
        in("a4") child_tid,
        options(nostack)
    );
    r0
}

/// Write a value to the platform thread-pointer register.
#[cfg(all(
    feature = "origin-thread",
    any(feature = "origin-start", feature = "external-start")
))]
#[inline]
pub(super) unsafe fn set_thread_pointer(ptr: *mut c_void) {
    asm!("mv tp,{}", in(reg) ptr);
    debug_assert_eq!(get_thread_pointer(), ptr);
}

/// Read the value of the platform thread-pointer register.
#[cfg(all(
    feature = "origin-thread",
    any(feature = "origin-start", feature = "external-start")
))]
#[inline]
pub(super) fn get_thread_pointer() -> *mut c_void {
    let ptr;
    unsafe {
        asm!("mv {},tp", out(reg) ptr, options(nostack, preserves_flags, readonly));
    }
    ptr
}

/// TLS data starts 0x800 bytes below the location pointed to by the thread
/// pointer.
#[cfg(feature = "origin-thread")]
pub(super) const TLS_OFFSET: usize = 0x800;

/// `munmap` the current thread, then carefully exit the thread without
/// touching the deallocated stack.
#[cfg(feature = "origin-thread")]
#[inline]
pub(super) unsafe fn munmap_and_exit_thread(map_addr: *mut c_void, map_len: usize) -> ! {
    asm!(
        "ecall",
        "mv a0,zero",
        "li a7,{__NR_exit}",
        "ecall",
        "unimp",
        __NR_exit = const __NR_exit,
        in("a7") __NR_munmap,
        in("a0") map_addr,
        in("a1") map_len,
        options(noreturn, nostack)
    );
}

// RISC-V doesn't use `__NR_rt_sigreturn`
