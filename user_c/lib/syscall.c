#include <stddef.h>
#include <unistd.h>

#include <syscall.h>

ssize_t write(int fd, const void* buf, size_t len) {
    return syscall(SYSCALL_WRITE, fd, (int64)buf, len);
}
ssize_t read(int fd, void* buf, size_t len) {
    return syscall(SYSCALL_READ, fd, buf, len);
}

void exit(int exit_code) {
    syscall(SYSCALL_EXIT, (int64)exit_code, 0, 0);
}

int uname(void* buf){
    return syscall(SYSCALL_UNAME, buf);
}