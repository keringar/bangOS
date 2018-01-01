use super::{ActivePageTable, Page, VirtualAddress};
use super::table::{Level1, Table};
use memory::{Frame, FrameAllocator};

pub struct TemporaryPage {
    page: Page,
    allocator: TinyAllocator,
}

impl TemporaryPage {
    /// Maps the temporary page to the given frame in the active table.
    /// Returns the start address of the temporary page.
    pub fn map(&mut self, frame: Frame, active_table: &mut ActivePageTable) -> VirtualAddress {
        use super::EntryFlags;

        assert!(
            active_table.translate_page(self.page).is_none(),
            "Temporary page is already mapped"
        );

        active_table.map_to(self.page, frame, EntryFlags::PRESENT | EntryFlags::WRITABLE, &mut self.allocator);
        self.page.start_address()
    }

    /// Maps the temporary page to the given table frame in the active table.
    /// Returns a reference to the newly mapped table.
    pub fn map_table_frame(
        &mut self,
        frame: Frame,
        active_table: &mut ActivePageTable,
    ) -> &mut Table<Level1> {
        unsafe { &mut *(self.map(frame, active_table) as *mut Table<Level1>) }
    }

    /// Unmaps the temporary page in the active page table
    pub fn unmap(&mut self, active_table: &mut ActivePageTable) {
        active_table.unmap(self.page, &mut self.allocator);
    }

    /// Make a new temporary page
    pub fn new<A>(page: Page, allocator: &mut A) -> TemporaryPage
    where
        A: FrameAllocator,
    {
        TemporaryPage {
            page,
            allocator: TinyAllocator::new(allocator),
        }
    }
}

/// To map a single temporary frame we just need three frames. A P3, P2 and P1, the P4 table is
/// always mapped. Therefore we don't need a full blown allocator and can just reuse some frames.
struct TinyAllocator([Option<Frame>; 3]);

impl TinyAllocator {
    /// Allocate a new TinyAllocator with some other allocator. Requires just three frames.
    pub fn new<A>(allocator: &mut A) -> TinyAllocator
    where
        A: FrameAllocator,
    {
        let mut f = || allocator.allocate_frame();
        let frames = [f(), f(), f()];
        TinyAllocator(frames)
    }
}

impl FrameAllocator for TinyAllocator {
    /// Take the first free frame and return it
    fn allocate_frame(&mut self) -> Option<Frame> {
        for frame_option in &mut self.0 {
            if frame_option.is_some() {
                return frame_option.take();
            }
        }
        None
    }

    /// Return a frame into the first free slot
    fn deallocate_frame(&mut self, frame: Frame) {
        for frame_option in &mut self.0 {
            if frame_option.is_none() {
                *frame_option = Some(frame);
                return;
            }
        }

        panic!("Tiny allocator is full");
    }
}
