#![feature(lang_items)]
#![feature(const_fn)]
#![feature(unique)]
#![no_std]

#[macro_use]
extern crate bitflags;
extern crate multiboot2;
extern crate rlibc;
extern crate spin;
extern crate volatile;
extern crate x86_64;

#[macro_use]
mod vga_buffer;
mod memory;

use memory::map::KERNEL_VMA;
use multiboot2::BootInformation;

#[no_mangle]
pub extern "C" fn rust_main(multiboot_info_addr: usize) {
    let boot_info = unsafe { BootInformation::load(multiboot_info_addr, KERNEL_VMA) };

    memory::init(&boot_info);

    println!("Hello world");

    loop {}
}

#[lang = "panic_fmt"]
#[no_mangle]
pub extern "C" fn panic_fmt(fmt: core::fmt::Arguments, file: &'static str, line: u32) -> ! {
    println!("\n\nPanic in {} at line {}:", file, line);
    println!("    {}", fmt);
    loop {}
}
