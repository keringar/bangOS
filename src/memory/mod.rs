mod area_frame_allocator;
pub use self::area_frame_allocator::AreaFrameAllocator;

mod paging;
use self::paging::PhysicalAddress;

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
}
