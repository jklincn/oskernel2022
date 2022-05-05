#include "stdio.h"
#include "unistd.h"
#include "stdlib.h"

#define PROG_NUM 6

char* prog_name[PROG_NUM] = { "c_uname", "c_fork", "c_exit", "c_exec", "c_getpid" , "c_getppid" };

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
            printf(COLOR_YELLOW"%s\t"COLOR_NONE, prog_name[t]);
            if (child_return != 0) {
                printf(COLOR_LIGHT_RED"TEST ERROR:%s return code:%d\n"COLOR_NONE, prog_name[t], child_return);
                return -t;
            }
            else{
                printf(COLOR_LIGHT_GREEN"OK\n"COLOR_NONE);
            }
        }
    }
    return 0;
}