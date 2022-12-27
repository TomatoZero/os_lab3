#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points
#![feature(abi_x86_interrupt)]

use core::fmt::Write;
use core::panic::PanicInfo;
use core::ptr::write;
use pc_keyboard::DecodedKey;
use crate::vga_buf::SCREEN;

mod vga_buf;
mod interrupts;
mod shell;

/// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    println!("----------------------------------------------");
    println!("{}", _info);
    println!("----------------------------------------------");
    loop {}
}

fn my_keyboard_handler(key: DecodedKey) {
    shell::handle_keyboard_interrupt(key);
}

fn my_timer_handler() {
    // print!(".")
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    interrupts::set_keyboard_interrupt_handler(my_keyboard_handler);
    interrupts::set_timer_interrupt_handler(my_timer_handler);
    interrupts::init();
    loop {}
}
