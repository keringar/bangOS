use memory::{Frame, FrameAllocator};
use multiboot2::{MemoryArea, MemoryAreaIter};

// Basic memory allocator based on Multiboot mapped memory. Chooses new frames by simply looking
// for the next multiboot provided memory area that is not already used by the kernel or the
// multiboot info struct. All memory below the next_free_frame is considered in use and cannot be
// freed.
pub struct AreaFrameAllocator {
    next_free_frame: Frame,
    current_area: Option<&'static MemoryArea>,
    areas: MemoryAreaIter,
    kernel_start: Frame,
    kernel_end: Frame,
    multiboot_start: Frame,
    multiboot_end: Frame,
}

impl AreaFrameAllocator {
    pub fn new(
        kernel_start: usize,
        kernel_end: usize,
        multiboot_start: usize,
        multiboot_end: usize,
        memory_areas: MemoryAreaIter,
    ) -> AreaFrameAllocator {
        let mut allocator = AreaFrameAllocator {
            next_free_frame: Frame::containing_address(0),
            current_area: None,
            areas: memory_areas,
            kernel_start: Frame::containing_address(kernel_start),
            kernel_end: Frame::containing_address(kernel_end),
            multiboot_start: Frame::containing_address(multiboot_start),
            multiboot_end: Frame::containing_address(multiboot_end),
        };

        allocator.choose_next_area();
        allocator
    }

    // Choose the next area to allocate frome
    fn choose_next_area(&mut self) {
        // Get the smallest next area that contains frames bigger than the current next_free_frame
        self.current_area = self.areas
            .clone()
            .filter(|area| {
                let address = area.base_addr + area.length - 1;
                Frame::containing_address(address as usize) >= self.next_free_frame
            })
            .min_by_key(|area| area.base_addr);

        // Iff a new area bigger than the current max was found, update the next_free_frame to
        // point to the first frame of the new area.
        if let Some(area) = self.current_area {
            let start_frame = Frame::containing_address(area.base_addr as usize);
            if self.next_free_frame < start_frame {
                self.next_free_frame = start_frame;
            }
        }
    }
}

impl FrameAllocator for AreaFrameAllocator {
    fn allocate_frame(&mut self) -> Option<Frame> {
        if let Some(area) = self.current_area {
            // Get the next available frame
            let frame = Frame {
                number: self.next_free_frame.number,
            };

            // Get the last frame that is still in the current area
            let last_frame_from_current_area = {
                let address = area.base_addr + area.length - 1;
                Frame::containing_address(address as usize)
            };

            if frame > last_frame_from_current_area {
                // All frames from the current area are in use, switch to the next area
                self.choose_next_area();
            } else if frame >= self.kernel_start && frame <= self.kernel_end {
                // Frame is in use by the kernel, skip to the next frame after
                self.next_free_frame = Frame {
                    number: self.kernel_end.number + 1,
                }
            } else if frame >= self.multiboot_start && frame <= self.multiboot_end {
                // Frame is in use by the multiboot info struct, skip to the next frame after
                self.next_free_frame = Frame {
                    number: self.multiboot_end.number + 1,
                }
            } else {
                // A new frame was found that was not in use
                self.next_free_frame.number += 1;
                return Some(frame);
            }

            // The frame wasn't valid and we had to skip reserved memory or go to some new area
            // try again with an updated next_free_frame
            self.allocate_frame()
        } else {
            // No free frames left
            None
        }
    }

    fn deallocate_frame(&mut self, _frame: Frame) {
        unimplemented!()
    }
}
