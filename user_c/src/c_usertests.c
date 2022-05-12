#include "stdio.h"
#include "unistd.h"
#include "stdlib.h"

#define PROG_NUM 14

char* prog_name[] = {"mount","umount","read","fstat","getdents", "getcwd", "open", "unlink" , "openat", "write","close", "mkdir_", "chdir","pipe"};

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
                return -t;
            }
            else{
            }
        }
    }
    return 0;
}