mod area_frame_allocator;
mod paging;
pub mod map;

pub use self::area_frame_allocator::AreaFrameAllocator;

use self::paging::remap_the_kernel;
use self::paging::PhysicalAddress;
use multiboot2::BootInformation;

pub const PAGE_SIZE: usize = 4096;

// Allocates physical memory
pub trait FrameAllocator {
    fn allocate_frame(&mut self) -> Option<Frame>;
    fn deallocate_frame(&mut self, frame: Frame);
}

// Represents a physical frame of memory
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Frame {
    number: usize,
}

impl Frame {
    fn start_address(&self) -> PhysicalAddress {
        self.number * PAGE_SIZE
    }

    fn containing_address(address: usize) -> Frame {
        Frame {
            number: address / PAGE_SIZE,
        }
    }

    // Private because the only way to get a frame should be from a FrameAllocator
    fn clone(&self) -> Frame {
        Frame {
            number: self.number,
        }
    }

    fn range_inclusive(start: Frame, end: Frame) -> FrameIter {
        FrameIter { start, end }
    }
}

struct FrameIter {
    start: Frame,
    end: Frame,
}

impl Iterator for FrameIter {
    type Item = Frame;

    fn next(&mut self) -> Option<Frame> {
        if self.start <= self.end {
            let frame = self.start.clone();
            self.start.number += 1;

            Some(frame)
        } else {
            None
        }
    }
}

pub fn init(boot_info: &BootInformation) {
    let memory_map = boot_info.memory_map().expect("Memory map tag required");
    let elf_sections = boot_info.elf_sections().expect("ELF sections tag required");

    // Contains the start and end values of the kernel
    extern "C" {
        static _higher_start: u8;
        static _end: u8;
    }

    // Drop the bootstrap code which lives before _higher_start
    let kernel_start = unsafe { ((&_higher_start as *const u8) as *const usize) as usize };
    let kernel_end = unsafe { ((&_end as *const u8) as *const usize) as usize };

    let mut frame_allocator = AreaFrameAllocator::new(
        kernel_start as usize,
        kernel_end as usize,
        boot_info.start_address() as usize,
        boot_info.end_address() as usize,
        memory_map.memory_areas(),
    );

    enable_nxe_bit();
    enable_write_protect_bit();

    remap_the_kernel(&mut frame_allocator, &boot_info);
}

fn enable_nxe_bit() {
    use x86_64::registers::msr::{IA32_EFER, rdmsr, wrmsr};

    let nxe_bit = 1 << 11;
    unsafe {
        let efer = rdmsr(IA32_EFER);
        wrmsr(IA32_EFER, efer | nxe_bit);
    }
}

fn enable_write_protect_bit() {
    use x86_64::registers::control_regs::{cr0, cr0_write, Cr0};

    unsafe {
        cr0_write(cr0() | Cr0::WRITE_PROTECT);
    }
}