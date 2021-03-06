mod entry;
mod mapper;
mod table;
mod temporary_page;

use core::ops::{Deref, DerefMut};
use memory::{Frame, FrameAllocator, PAGE_SIZE};
pub use self::entry::{Entry, EntryFlags};
use self::temporary_page::TemporaryPage;
use self::mapper::Mapper;
use super::map::{KERNEL_VMA, RECURSIVE_ENTRY, TEMP_PAGE, VGA_BUFFER_VMA};
use multiboot2::BootInformation;

// Number of entries per page table
const ENTRY_COUNT: usize = 512;

pub type PhysicalAddress = usize;
pub type VirtualAddress = usize;

/// Remap the kernel sections properly
pub fn remap_the_kernel<A>(allocator: &mut A, boot_info: &BootInformation)
where
    A: FrameAllocator,
{
    // Contains the initial page tables from bootstrap
    let mut active_table = unsafe { ActivePageTable::new() };

    // Allocate space for a new set of page tables and set up a new inactive page table
    let mut temporary_page = TemporaryPage::new(Page::containing_address(TEMP_PAGE), allocator);
    let mut new_table = {
        let frame = allocator.allocate_frame().expect("No more free frames");
        InactivePageTable::new(frame, &mut active_table, &mut temporary_page)
    };

    // Get the address of the guard page
    extern "C" {
        static _guard_page: u8;
    }
    let guard_page_addr = unsafe { ((&_guard_page as *const u8) as *const usize) as usize };
    assert!(
        guard_page_addr % PAGE_SIZE == 0,
        "Guard page is not page aligned"
    );

    // Map the new page table
    active_table.with(&mut new_table, &mut temporary_page, |mapper| {
        let elf_sections_tag = boot_info.elf_sections().expect("Memory map tag required");

        // Map kernel elf sections except for the stack guard page
        for section in elf_sections_tag.sections() {
            // Skip sections that aren't allocated (i.e. debugging sections) or before the start
            // of the higher half and therefore not part of the kernel.
            if !section.is_allocated() || (section.start_address() < KERNEL_VMA) {
                continue;
            }

            assert!(
                section.start_address() % PAGE_SIZE == 0,
                "Sections must be page aligned"
            );

            let flags = EntryFlags::from_elf_section(section);
            let start_frame = Frame::containing_address(section.start_address());
            let end_frame = Frame::containing_address(section.end_address() - 1);

            for frame in Frame::range_inclusive(start_frame, end_frame) {
                let virtual_address = frame.start_address();
                let physical_address = virtual_address - KERNEL_VMA;

                mapper.map_to(
                    Page::containing_address(virtual_address),
                    Frame::containing_address(physical_address),
                    flags,
                    allocator,
                );
            }
        }

        // Map the frame buffer
        mapper.map_to(
            Page::containing_address(VGA_BUFFER_VMA),
            Frame::containing_address(0xb8000),
            EntryFlags::WRITABLE,
            allocator,
        );

        // Map the multiboot structure
        let multiboot_start = Frame::containing_address(boot_info.start_address());
        let multiboot_end = Frame::containing_address(boot_info.end_address() - 1);

        for frame in Frame::range_inclusive(multiboot_start, multiboot_end) {
            mapper.map_to(
                Page::containing_address(frame.start_address()),
                Frame::containing_address(frame.start_address() - KERNEL_VMA),
                EntryFlags::PRESENT,
                allocator,
            );
        }

        // Unmap the guard page
        mapper.unmap(Page::containing_address(guard_page_addr), allocator);
    });

    active_table.switch(new_table);
}

pub struct ActivePageTable {
    mapper: Mapper,
}

impl Deref for ActivePageTable {
    type Target = Mapper;

    fn deref(&self) -> &Mapper {
        &self.mapper
    }
}

impl DerefMut for ActivePageTable {
    fn deref_mut(&mut self) -> &mut Mapper {
        &mut self.mapper
    }
}

impl ActivePageTable {
    unsafe fn new() -> ActivePageTable {
        ActivePageTable {
            mapper: Mapper::new(),
        }
    }

    /// Temporarily map the inactive table and run the closure
    pub fn with<F>(
        &mut self,
        table: &mut InactivePageTable,
        temporary_page: &mut temporary_page::TemporaryPage,
        f: F,
    ) where
        F: FnOnce(&mut Mapper),
    {
        use x86_64::instructions::tlb;
        use x86_64::registers::control_regs;

        // Inner scope to end the borrow of "temporary page"
        {
            // Backup the current P4 and temporarily remap it
            let original_p4 = Frame::containing_address(control_regs::cr3().0 as usize);
            let p4_table = temporary_page.map_table_frame(original_p4.clone(), self);

            // Overwrite recursive mapping
            self.p4_mut()[RECURSIVE_ENTRY].set(
                table.p4_frame.clone(),
                EntryFlags::PRESENT | EntryFlags::WRITABLE,
            );
            tlb::flush_all();

            f(self);

            // Restore recursive mapping to original P4 table
            p4_table[RECURSIVE_ENTRY].set(original_p4, EntryFlags::PRESENT | EntryFlags::WRITABLE);
            tlb::flush_all();
        }

        temporary_page.unmap(self);
    }

    /// Switch the active page table with the passed in page table. Returns the old page table.
    pub fn switch(&mut self, new_table: InactivePageTable) -> InactivePageTable {
        use x86_64::registers::control_regs;

        let old_table = InactivePageTable {
            p4_frame: Frame::containing_address(control_regs::cr3().0 as usize),
        };

        unsafe {
            let address = new_table.p4_frame.start_address() as usize;
            asm!("mov $0, %cr3" :: "r" (address));
        }

        old_table
    }
}

#[derive(Debug)]
pub struct InactivePageTable {
    p4_frame: Frame,
}

impl InactivePageTable {
    pub fn new(
        frame: Frame,
        active_table: &mut ActivePageTable,
        temporary_page: &mut TemporaryPage,
    ) -> InactivePageTable {
        {
            let table = temporary_page.map_table_frame(frame.clone(), active_table);
            table.zero();
            table[RECURSIVE_ENTRY].set(frame.clone(), EntryFlags::PRESENT | EntryFlags::WRITABLE);
        }

        temporary_page.unmap(active_table);

        InactivePageTable { p4_frame: frame }
    }
}

// Represents a virtual page of memory
#[derive(Debug, Clone, Copy)]
pub struct Page {
    number: usize,
}

impl Page {
    pub fn containing_address(address: VirtualAddress) -> Page {
        assert!(
            address < 0x0000_8000_0000_0000 || address >= 0xffff_8000_0000_0000,
            "invalid adress: 0x{:x}",
            address
        );
        Page {
            number: address / PAGE_SIZE,
        }
    }

    fn start_address(&self) -> usize {
        self.number * PAGE_SIZE
    }

    fn p4_index(&self) -> usize {
        (self.number >> 27) & 0o777
    }

    fn p3_index(&self) -> usize {
        (self.number >> 18) & 0o777
    }

    fn p2_index(&self) -> usize {
        (self.number >> 9) & 0o777
    }

    fn p1_index(&self) -> usize {
        (self.number >> 0) & 0o777
    }
}
