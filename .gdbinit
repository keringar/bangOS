layout asm
target remote localhost:1234
symbol-file ./build/kernel-x86_64.bin
b bang::rust_main
continue

