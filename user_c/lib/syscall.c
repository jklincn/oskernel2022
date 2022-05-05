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
int uname(void* buf) {
    return syscall(SYSCALL_UNAME, buf);
}
pid_t fork(void) {
    return syscall(SYSCALL_FORK, SIGCHLD, 0);
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
pid_t getpid(void){
    return syscall(SYSCALL_GETPID);
}
pid_t getppid(void){
    return syscall(SYSCALL_GETPPID);
}