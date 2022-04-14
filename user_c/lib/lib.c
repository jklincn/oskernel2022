#include"syscall.h"

int __attribute__((weak)) main()
{
    char* s = "Unreachable\n";
    sys_write(FD_STDOUT, s, 13);
    return 0;
}

__attribute__((section(".text.entry"))) void _start()
{
    sys_exit(main());
}