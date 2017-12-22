global start
extern long_mode_start

section .text
bits 32
start:
    ; Update stack pointer to point to start of stack memory
    mov esp, stack_top

    ; Move Multiboot info pointer to edi
    mov edi, ebx

    ; Check for required cpu support, erroring if not supported
    call check_multiboot
    call check_cpuid
    call check_long_mode

    call set_up_page_tables
    call enable_paging

    ; load the 64-bit GDT
    lgdt [gdt64.pointer]

    ; Far jump to 64 bit code
    jmp gdt64.code:long_mode_start
    
    ; print 'OK' to screen
    mov dword [0xb8000], 0x2f4b2f4f
    hlt

; Check if the kernel was booted by a multiboot compliant bootloader. A multiboot compilant
; bootloader must write the magic value 0x36d76289 to the eax register
check_multiboot:
    cmp eax, 0x36d76289
    jne .no_multiboot
    ret
.no_multiboot:
    mov al, "0"
    jmp error

; Check if CPUID is supported. Copied from the OSdev wiki
check_cpuid:
    ; Check if CPUID is supported by attempting to flip the ID bit (bit 21) in
    ; the FLAGS register. If we can flip it, CPUID is available.
 
    ; Copy FLAGS in to EAX via stack
    pushfd
    pop eax
 
    ; Copy to ECX as well for comparing later on
    mov ecx, eax
 
    ; Flip the ID bit
    xor eax, 1 << 21
 
    ; Copy EAX to FLAGS via the stack
    push eax
    popfd
 
    ; Copy FLAGS back to EAX (with the flipped bit if CPUID is supported)
    pushfd
    pop eax
 
    ; Restore FLAGS from the old version stored in ECX (i.e. flipping the ID bit
    ; back if it was ever flipped).
    push ecx
    popfd
 
    ; Compare EAX and ECX. If they are equal then that means the bit wasn't
    ; flipped, and CPUID isn't supported.
    cmp eax, ecx
    je .no_cpuid
    ret
.no_cpuid:
    mov al, "1"
    jmp error

; Check if long mode is supported, basically 64 bit mode
check_long_mode:
    ; test if extended processor info in available
    mov eax, 0x80000000    ; implicit argument for cpuid
    cpuid                  ; get highest supported argument
    cmp eax, 0x80000001    ; it needs to be at least 0x80000001
    jb .no_long_mode       ; if it's less, the CPU is too old for long mode

    ; use extended info to test if long mode is available
    mov eax, 0x80000001    ; argument for extended processor info
    cpuid                  ; returns various feature bits in ecx and edx
    test edx, 1 << 29      ; test if the LM-bit is set in the D-register
    jz .no_long_mode       ; If it's not set, there is no long mode
    ret
.no_long_mode:
    mov al, "2"
    jmp error

set_up_page_tables:
    ; Map the last entry of P4 to itself
    mov eax, p4_table
    or eax, 0b11 ; set PRESENT + WRITABLE
    mov [p4_table + 511 * 8], eax

    ; map first P4 entry to P3 table
    mov eax, p3_table
    or eax, 0b11 ; set PRESENT + WRITABLE
    mov [p4_table], eax

    ; map first P3 entry to P2 table
    mov eax, p2_table
    or eax, 0b11 ; set PRESENT + WRITABLE
    mov [p3_table], eax

    ; map each p2 entry to a 2 MiB page
    mov ecx, 0 ; counter variable
.map_p2_table:
    ; map the ecx-th P2 entry to a huge page that starts at address 2 MiB * ecs
    mov eax, 0x200000   ; 2 MiB
    mul ecx             ; start address of the ecx-th page
    or eax, 0b10000011  ; Start address of each ecx-th page
    mov [p2_table + ecx * 8], eax ; map ecx-th entry

    inc ecx           ; increment counter
    cmp ecx, 512      ; if counter == 512
    jne .map_p2_table ; then map the next entry

    ret

enable_paging:
    ; load P4 to to CR3 register
    mov eax, p4_table
    mov cr3, eax

    ; enable PAE-flag in cr4 (Physical Address Extension)
    mov eax, cr4
    or eax, 1 << 5
    mov cr4, eax

    ; set the long mode bit in the EFER MSR
    mov ecx, 0xC0000080
    rdmsr
    or eax, 1 << 8
    wrmsr

    ; enable paging in the cr0 register
    mov eax, cr0
    or eax, 1 << 31
    mov cr0, eax

    ret

; Prints 'ERR: ' and the given error code to the screen and hangs
; Param: ASCII error code in al register
error:
    mov dword [0xb8000], 0x4f524f45
    mov dword [0xb8004], 0x4f3a4f52
    mov dword [0xb8008], 0x4f204f20
    mov byte  [0xb800a], al
    hlt

section .bss
; Ensure page tables are page aligned
align 4096
p4_table:
    resb 4096
p3_table:
    resb 4096
p2_table:
    resb 4096
; Reserve stack space
stack_bottom:
    resb 4096 * 4
stack_top:

section .rodata
gdt64:
    dq 0 ; zero entry
.code: equ $ - gdt64
    dq (1<<43) | (1<<44) | (1<<47) | (1<<53) ; code segment
.pointer:
    dw $ - gdt64 - 1
    dq gdt64