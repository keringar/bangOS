// The bootstrap code maps the kernel at the higher-half at 0xfffffff80000000 + (Kernel physical base)
// It maps one P2 table of huge 2 MiB pages, so 1 GiB from 0xffffffff80000000 to 0xffffffffc0000000

// The kernel is mapped at 0xffffffff80000000 + {Kernel Physical Address}. This is located at
// P4 = 511, P3 = 510, P2 = 0 and P1 = 0 onwards. Since P4[511] is already used, we recursively map
// P4 at 510 instead. Everything else from 0x0 to 0xffffff7fffffff can be used by user mode.
pub const RECURSIVE_ENTRY: usize = 510;

// 0xFFFF + (510 << 39) + (510 << 30) + (510 << 21) + (510 << 12)
pub const P4_TABLE_ADDRESS: usize = 0o177777_000_000_000_000_0000 + (RECURSIVE_ENTRY<<39) // P4 slot
                                                                  + (RECURSIVE_ENTRY<<30) // P3 slot
                                                                  + (RECURSIVE_ENTRY<<21) // P2 slot
                                                                  + (RECURSIVE_ENTRY<<12) // P1 slot
                                                                  + (0<<0); // Offset

pub const KERNEL_VMA: usize = 0xffffffff80000000;
pub const VGA_BUFFER_VMA: usize = 0xffffffff80000000 + 0xb8000;

// 0xffffffffc0000000
pub const HEAP_START: usize = KERNEL_VMA + 0o0000010000000000;
pub const HEAP_SIZE: usize = 100 * 1024;

// 0xfffffffff0000000
pub const TEMP_PAGE: usize = 0xfffffffff0000000;
