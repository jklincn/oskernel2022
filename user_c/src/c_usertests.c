#include "stdio.h"
#include "unistd.h"
#include "stdlib.h"

#define PROG_NUM 11

char* prog_name[] = { "fork", "exit", "execve", "getpid" , "sleep", "gettimeofday", "dup", "times", "user_shell", "c_getppid","uname"  };

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