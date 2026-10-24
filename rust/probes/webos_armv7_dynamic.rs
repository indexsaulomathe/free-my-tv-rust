#![no_std]
#![no_main]

use core::arch::global_asm;

global_asm!(
    ".syntax unified",
    ".arm",
    ".global _start",
    ".type _start, %function",
    "_start:",
    "bl rust_probe",
    "mov r7, #1", // Linux ARM EABI exit syscall; r0 holds the Rust return value.
    "svc #0",
    ".size _start, .-_start",
);

extern "C" {
    fn getpid() -> i32;
}

#[no_mangle]
pub extern "C" fn rust_probe() -> i32 {
    if unsafe { getpid() } > 0 {
        42
    } else {
        1
    }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    loop {}
}
