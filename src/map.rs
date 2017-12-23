pub const RECURSIVE_ENTRY: usize = 510;
pub const P4_TABLE_ADDRESS: usize = 0o177777_000_000_000_000_0000 + (RECURSIVE_ENTRY<<39) // P4 slot
                                                                  + (RECURSIVE_ENTRY<<30) // P3 slot
                                                                  + (RECURSIVE_ENTRY<<21) // P2 slot
                                                                  + (RECURSIVE_ENTRY<<12) // P1 slot
                                                                  + (0<<0); // Offset

pub const KERNEL_VMA: usize = 0xffffffff80000000;
pub const VGA_BUFFER_VMA: usize = 0xffffffff800b8000;
