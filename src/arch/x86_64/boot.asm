global start

; boot.asm is physically mapped and responsible for entering Long Mode and jumping into the kernel
; Constants for addressing and creating page tables
KERNEL_VMA equ 0xFFFFFFFF80000000
PAGE_SIZE equ 0x1000 ; 4 KiB
PAGE_PRESENT equ 0x1
PAGE_WRITABLE equ 0x2
PAGE_USER equ 0x4
PAGE_HUGE equ 0x80
PAGE_NO_EXEC equ 0x8000000000000000

; === BOOTSTRAP DATA === 
section .bootstrap_data
align 4096
; Page map with both an identity map and a higher side identity map
p4_table:
    dq (p3_table + PAGE_PRESENT + PAGE_WRITABLE) ; 0 - Identity map
    times (512 - 3) dq 0                        ; ...
    dq (p4_table + PAGE_PRESENT + PAGE_WRITABLE) ; 510 - Recursive mapping of P4
    dq (p3_table_higher + PAGE_PRESENT + PAGE_WRITABLE) ; 511 - Higher half kernel map
p3_table:
    dq (p2_table + PAGE_PRESENT + PAGE_WRITABLE) ; 0
    dq 0                                        ; 1
    times (512 - 2) dq 0                        ; ...
p3_table_higher:
    times (512 - 2) dq 0                        ; ...
    dq (p2_table + PAGE_PRESENT + PAGE_WRITABLE) ; 510 - Identity map
    dq 0                                        ; 511
p2_table: ; Map each page entry with 2 MiB huge pages
    %assign pg 0
    %rep 512
        dq (pg + PAGE_PRESENT + PAGE_WRITABLE + PAGE_HUGE)
        %assign pg pg+0x200000
    %endrep

; Global Descriptor Table
gdt64:
    dq 0 ; zero entry
.code: equ $ - gdt64
    dq (1<<43) | (1<<44) | (1<<47) | (1<<53) ; code segment
.pointer:
    dw $ - gdt64 - 1
    dq gdt64
.pointer_high:
    dw .pointer - gdt64 - 1
    dq gdt64 + KERNEL_VMA

; Initial stack
bootstrap_stack_bottom:
    times 64 db 0 ; 64 bytes of temporary stack space
bootstrap_stack_top:


; === BOOTSTRAP CODE === 
section .bootstrap
align 4096
bits 32
start:
    ; Update stack pointer to point to the start of bootstrap stack
    mov esp, bootstrap_stack_top

    ; Move Multiboot info pointer to edi
    mov edi, ebx

    call clear_screen

    ; Check for required cpu support, erroring if not supported
    call check_multiboot
    call check_cpuid
    call check_long_mode

    call enable_paging

    ; load the 64-bit GDT
    lgdt [gdt64.pointer]

    ; Far jump to a trampoline to 64 bit code
    jmp gdt64.code:long_mode_trampoline

; Clear the vga buffer to black
clear_screen:
    pusha
    mov eax, 0x20
    mov edi, 0xb8000
    mov ecx, 80 * 25
    cld
    rep stosw
    popa
    ret

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
    pushfd
    pop eax
    mov ecx, eax
    xor eax, 1 << 21
    push eax
    popfd
    pushfd
    pop eax
    push ecx
    popfd
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

bits 64
long_mode_trampoline:
    mov rax, qword long_mode_start
    jmp rax

section .text
bits 64
align 4096
extern rust_main

long_mode_start:
    ; Reload the GDT pointer with the correct virtual address
    lgdt [gdt64.pointer_high + KERNEL_VMA]

    mov ax, 0
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    
    ; Set up the actual stack
    mov rbp, stack_bottom ; Terminate stack traces here, can't cross from higher to lower addresses
    mov rsp, stack_top

    ; Unmap the identity map
    mov qword [p4_table], 0x0
    invlpg [0x0]

    ; Call into the kernel proper
    call rust_main

    ; Halt if we ever return
    hlt

section .bss
align 4096
global _guard_page
_guard_page:
    resb 4096 ; Intentionally unmapped
stack_bottom:
    resb 4096 * 4 ; 16 KiB stack space
stack_top: