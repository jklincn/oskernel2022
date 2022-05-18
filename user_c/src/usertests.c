#include "stdio.h"
#include "unistd.h"
#include "stdlib.h"

#define PROG_NUM 17

char* prog_name[] = {  "mmap", "clone", "yield", "waitpid", "dup2", "dup", "exec", "exit", "fork", "getpid", "gettimeofday", "uname", "sleep", "times", "pipe", "wait", "open"};

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
            if (child_return != 0) {
                printf(COLOR_LIGHT_RED"TEST ERROR:%s return code:%d"COLOR_NONE"\n", prog_name[t], child_return);
                //return -t;
            }
            else{
                printf(COLOR_LIGHT_GREEN"OK"COLOR_NONE"\n");
            }
        }
    }
    return 0;
}