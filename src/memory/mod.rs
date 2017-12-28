mod area_frame_allocator;
mod paging;
pub mod map;

pub use self::area_frame_allocator::AreaFrameAllocator;
pub use self::paging::remap_the_kernel;

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
