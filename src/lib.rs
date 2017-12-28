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

    let memory_map = boot_info.memory_map().expect("Memory map tag required");

    extern "C" {
        static _higher_start: u8;
        static _end: u8;
    }

    let kernel_start = unsafe { ((&_higher_start as *const u8) as *const usize) as usize };
    let kernel_end = unsafe { ((&_end as *const u8) as *const usize) as usize };

    println!("Loaded kernel to 0x{:x} - 0x{:x}", kernel_start, kernel_end);
    println!(
        "Boot information at: 0x{:x} - 0x{:x}",
        boot_info.start_address(),
        boot_info.end_address()
    );

    let mut frame_allocator = memory::AreaFrameAllocator::new(
        kernel_start as usize,
        kernel_end as usize,
        boot_info.start_address() as usize,
        boot_info.end_address() as usize,
        memory_map.memory_areas(),
    );

    memory::remap_the_kernel(&mut frame_allocator, &boot_info);

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
