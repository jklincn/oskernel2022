/* user/src/linker.ld */
OUTPUT_ARCH(riscv)
ENTRY(_start)
BASE_ADDRESS = 0x80400000;  /*不同于os的基地址*/
/*使用build.py脚本编译用户程序时会改变这一值*/

SECTIONS
{
    . = BASE_ADDRESS;
    .text : {
        *(.text.entry)
        *(.text .text.*)
    }
    .rodata : {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }
    .data : {
        *(.data .data.*)
        *(.sdata .sdata.*)
    }
    .bss : {
        start_bss = .;  /*提供给lib.rs::clear_bss*/
        *(.bss.stack)
        *(.bss .bss.*)
        end_bss = .;    /*提供给lib.rs::clear_bss*/
    }
    /DISCARD/ : {
        *(.eh_frame)
        *(.debug*)
    }
}