#include "stdio.h"
#include "unistd.h"
#include "stdlib.h"

#define PROG_NUM 32

char* prog_name[] = { "mmap", "dup", "dup2", "execve", "exit", "fork", "getpid", "getppid", "gettimeofday", "uname", "sleep", "times", "pipe", "wait", "waitpid",
    "open",  "openat", "close",  "read",  "write", "mount", "umount", "mkdir_",  "chdir", "unlink", "fstat", "getcwd", "getdents",  "yield" , "clone", "brk",
    "munmap" };

int main() {
    for (int t = 0; t < PROG_NUM; t++) {
        int npid = fork();
        assert(npid >= 0);

        int child_return;
        if (npid == 0) { //子进程
            exec(prog_name[t]);
        }
        else {          // 父进程
            child_return = -1;
            waitpid(npid, &child_return, 0);
        }
    }
    return 0;
}