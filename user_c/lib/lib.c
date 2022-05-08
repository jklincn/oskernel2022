#include <stdio.h>
#include <unistd.h>

int __attribute__((weak)) main()
{
    char* s = "Unreachable\n";
    puts(s);
    return 0;
}

__attribute__((section(".text.entry"))) void _start()
{
    exit(main());
}