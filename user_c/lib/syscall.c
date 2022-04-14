#include "syscall.h"

typedef unsigned long long usize;
typedef long long isize;

isize syscall(usize id, usize a0, usize a1, usize a2)
{
    isize ret;
    asm volatile(
        "mv x10, %1\n"
        "mv x11, %2\n"
        "mv x12, %3\n"
        "mv x17, %4\n"
        "ecall\n"
        "mv %0, x10\n"
        : "=r"(ret)
        : "r"(a0), "r"(a1), "r"(a2), "r"(id)
        : "x10", "x11", "x12", "x17");
    return ret;
}

isize sys_write(usize fd, char* buf, usize len)
{
    return syscall(SYSCALL_WRITE, fd, (usize)buf, len);
}

isize sys_exit(int exit_code) {
    return syscall(SYSCALL_EXIT, (usize)exit_code, 0, 0);
}