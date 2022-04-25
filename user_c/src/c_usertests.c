#include "stdio.h"
#include "unistd.h"
#include "stdlib.h"

#define PROG_NUM 4

char* prog_name[PROG_NUM] = { "c_uname", "c_fork", "c_exit", "c_exec" };

int main() {
    for (int t = 0; t < PROG_NUM; t++) {
        int npid = fork();
        assert(npid >= 0);

        int child_return;
        if (npid == 0) { //子进程
            exec(prog_name[t]);
        }
        else {          // 父进程
            waitpid(npid, &child_return, 0);
            if (child_return != 0) {
                printf("TEST ERROR:%s", prog_name[t]);
                return -t;
            }
        }
    }
    return 0;
}