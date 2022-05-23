#include <stddef.h>
#include <unistd.h>

#include <syscall.h>

ssize_t read(int fd, void* buf, size_t len) {
    return syscall(SYS_read, fd, buf, len);
}

ssize_t write(int fd, const void* buf, size_t len) {
    return syscall(SYS_write, fd, buf, len);
}
void exit(int exit_code) {
    syscall(SYSCALL_EXIT, (int64)exit_code, 0, 0);
}
int uname(void* buf) {
    return syscall(SYSCALL_UNAME, buf);
}
int waitpid(int pid, int* code, int options) {
    return syscall(SYSCALL_WAITPID, pid, code, options, 0);
}
int wait(int* code) {
    return waitpid((int)-1, code, 0);
}
int sched_yield(void) {
    return syscall(SYSCALL_YIELD);
}
int exec(char* name) {
    char* newargv[] = { NULL, NULL };
    char* newenviron[] = { NULL };
    return syscall(SYSCALL_EXEC, name, newargv, newenviron);
}
int execve(const char* name, char* const argv[], char* const argp[]) {
    return syscall(SYSCALL_EXEC, name, argv, argp);
}
pid_t getpid(void) {
    return syscall(SYSCALL_GETPID);
}
pid_t getppid(void) {
    return syscall(SYSCALL_GETPPID);
}
int64 get_time() {
    TimeVal time;
    int err = sys_get_time(&time, 0);
    if (err == 0) {
        return ((time.sec & 0xffff) * 1000 + time.usec / 1000);
    }
    else {
        return -1;
    }
}
int sys_get_time(TimeVal* ts, int tz) {
    return syscall(SYSCALL_GET_TIME, ts, tz);
}
int sleep(unsigned long long time) {
    TimeVal tv = { .sec = time, .usec = 0 };
    if (syscall(SYSCALL_NANOSLEEP, &tv, &tv)) return tv.sec;
    return 0;
}
int times(void* mytimes) {
    return syscall(SYSCALL_TIMES, mytimes);
}
int dup(int fd) {
    return syscall(SYSCALL_DUP, fd);
}
int dup2(int old_fd, int new_fd) {
    return syscall(SYSCALL_DUP3, old_fd, new_fd);
}
int close(int fd) {
    return syscall(SYSCALL_CLOSE, fd);
}
int pipe(int* fd) {
    return syscall(SYSCALL_PIPE, fd, 0);
}
int open(const char* path, int flags)
{
    return syscall(SYS_openat, AT_FDCWD, path, flags, O_RDWR);
}
pid_t fork(void) {
    return syscall(SYSCALL_FORK, SIGCHLD, 0);
}
pid_t clone(int (*fn)(void* arg), void* arg, void* stack, size_t stack_size, unsigned long flags)
{
    if (stack)
        stack += stack_size;

    return __clone(fn, stack, flags, NULL, NULL, NULL);
    //return syscall(SYS_clone, flags, stack, NULL, NULL, NULL);
    //       syscall(SYS_clone, flags, stack, ptid, tls, ctid)
}
void* mmap(void* start, size_t len, int prot, int flags, int fd, off_t off) {
    return syscall(SYSCALL_MMAP, start, len, prot, flags, fd, off);
}
int munmap(void* start, size_t len) {
    return syscall(SYSCALL_MUNMAP, start, len);
}
int fstat(int fd, struct kstat* st) {
    return syscall(SYSCALL_FSTAT, fd, st);
}
int brk(void* addr) {
    return syscall(SYSCALL_BRK, addr);
}