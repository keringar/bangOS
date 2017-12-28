use memory::Frame;
use multiboot2::ElfSection;

// A single entry into the page table. An unused entry is defined to be 0
pub struct Entry(u64);

impl Entry {
    pub fn is_unused(&self) -> bool {
        self.0 == 0
    }

    pub fn set_unused(&mut self) {
        self.0 = 0;
    }

    pub fn flags(&self) -> EntryFlags {
        EntryFlags::from_bits_truncate(self.0)
    }

    pub fn pointed_frame(&self) -> Option<Frame> {
        if self.flags().contains(EntryFlags::PRESENT) {
            Some(Frame::containing_address(
                // Bitmask bits 12-51. i.e. physical address
                self.0 as usize & 0x000fffff_fffff000,
            ))
        } else {
            None
        }
    }

    pub fn set(&mut self, frame: Frame, flags: EntryFlags) {
        // Assert the address doesn't have any non-address bits set
        assert!(frame.start_address() & !0x000fffff_fffff000 == 0);
        self.0 = (frame.start_address() as u64) | flags.bits();
    }
}

bitflags! {
    pub struct EntryFlags: u64 {
        // Page is in memory
        const PRESENT         = 1 << 0;
        // Page is writable
        const WRITABLE        = 1 << 1;
        // Page allowed to be used by usermode
        const USER_ACCESSIBLE = 1 << 2;
        // Writes go directly to physical memory
        const WRITE_THROUGH   = 1 << 3;
        // No cache is used
        const NO_CACHE        = 1 << 4;
        // Set by CPU upon page access
        const ACCESSED        = 1 << 5;
        // Set by CPU when page is written to
        const DIRTY           = 1 << 6;
        // 1 GiB page in P3, 2 MiB page in P2. Else must be 0
        const HUGE_PAGE       = 1 << 7;
        // Page not flushed on address space switch
        const GLOBAL          = 1 << 8;
        // Forbid code execution
        const NO_EXECUTE      = 1 << 63;
    }
}

impl EntryFlags {
    pub fn from_elf_section(section: &ElfSection) -> EntryFlags {
        use multiboot2::{ELF_SECTION_ALLOCATED, ELF_SECTION_WRITABLE, ELF_SECTION_EXECUTABLE};
        let mut flags = EntryFlags::empty();

        if section.flags().contains(ELF_SECTION_ALLOCATED) {
            flags |= EntryFlags::PRESENT;
        }

        if section.flags().contains(ELF_SECTION_WRITABLE) {
            flags |= EntryFlags::WRITABLE;
        }

        if section.flags().contains(ELF_SECTION_EXECUTABLE) {
            flags |= EntryFlags::NO_EXECUTE;
        }

        flags
    }
}