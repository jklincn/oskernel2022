#include "stdio.h"
#include "unistd.h"
#include "stdlib.h"

#define PROG_NUM 29

char* prog_name[] = { "dup", "dup2", "execve", "exit", "fork", "getpid", "gettimeofday", "uname", "sleep", "times", "pipe", "wait", "waitpid",
    "open",  "openat", "close",  "read",  "write", "mount", "umount", "mkdir_",  "chdir", "unlink", "fstat", "getcwd", "getdents",  "yield" , "clone",
    "mmap"};

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