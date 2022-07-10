#include "stdio.h"
#include "unistd.h"
#include "stdlib.h"

#define PROG_NUM 32

char *prog_name[] = {"mmap", "dup", "dup2", "execve", "exit", "fork", "getpid", "getppid", "uname", "pipe", "wait", "waitpid", "munmap",
                     "open", "openat", "close", "read", "write", "mount", "umount", "mkdir_", "chdir", "unlink", "fstat", "getcwd", "getdents", "yield", "clone", "brk",
                     "gettimeofday", "sleep", "times"};

char *argv[] = {"-w", "entry-static.exe", "argv", 0};

int main()
{
    int npid = fork();
    assert(npid >= 0);

    int child_return;
    if (npid == 0)
    {   //子进程
        //exec("arg");
        execve("runtest.exe",argv,NULL);
    }
    else
    { // 父进程
        child_return = -1;
        waitpid(npid, &child_return, 0);
    }

    return 0;
}