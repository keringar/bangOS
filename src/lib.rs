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

use memory::FrameAllocator;

#[no_mangle]
pub extern "C" fn rust_main(multiboot_info_addr: usize) {
    let boot_info = unsafe { multiboot2::load(0xFFFFFFFF80000000 + multiboot_info_addr) };

    let memory_map_tag = boot_info.memory_map_tag().expect("Memory map tag required");
    let elf_sections_tag = boot_info
        .elf_sections_tag()
        .expect("Elf-sections tag required");
    let kernel_start = elf_sections_tag
        .sections()
        .map(|s| {
            if s.addr < 0xFFFFFFFF80000000 {
                s.addr + 0xFFFFFFFF80000000
            } else {
                s.addr
            }
        })
        .min()
        .unwrap();
    let kernel_end = elf_sections_tag
        .sections()
        .map(|s| s.addr + s.size)
        .max()
        .unwrap();
    let multiboot_start = multiboot_info_addr;
    let multiboot_end = multiboot_start + (boot_info.total_size as usize);

    let mut frame_allocator = memory::AreaFrameAllocator::new(
        kernel_start as usize,
        kernel_end as usize,
        multiboot_start as usize,
        multiboot_end as usize,
        memory_map_tag.memory_areas(),
    );

    println!("kernel_start: 0x{:x}", kernel_start);
    println!("kernel_end: 0x{:x}", kernel_end);
    println!("multiboot_start: 0x{:x}", multiboot_start);
    println!("multiboot_end: 0x{:x}", multiboot_end);

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
