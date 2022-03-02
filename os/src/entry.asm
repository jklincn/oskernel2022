# os/src/entry.asm    
    .section .text.entry    # 表明我们希望将第 2 行后面的内容全部放到一个名为 .text.entry 的段中
    .globl _start           # 声明了一个符号,符号 _start 的地址即为第 5 行的指令所在的地址
 _start:
    la sp, boot_stack_top   # x[sp] = &boot_stack_top
    call rust_main

    .section .bss.stack
    .globl boot_stack
boot_stack:                 # 用更低地址的符号boot_stack来标识栈底的位置
    .space 4096 * 16        # 在内核的内存布局中预留一块大小为4096*16字节的空间用作接下来要运行的程序的栈空间
    .globl boot_stack_top
boot_stack_top:             # 用更高地址的符号boot_stack_top来标识栈顶的位置