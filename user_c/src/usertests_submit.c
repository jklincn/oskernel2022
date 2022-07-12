#include "stdio.h"
#include "unistd.h"
#include "stdlib.h"

#define PROG_NUM 110

char proc_name[40];
char buf[6000];
char *argv[] = {"-w", "entry-static.exe", proc_name, 0};
int offset = 0;

void read_test_name()
{
    // skip space
    for (int k = 0; k < 3; k++)
    {
        for (; buf[offset] != ' '; offset++)
            ;
        offset++;
    }
    int k;
    for (k = 0; buf[offset] != '\n'; k++, offset++)
        *(proc_name + k) = buf[offset];
    *(proc_name + k + 1) = '\0';
    offset++;
}

int main()
{
    int fd = open("./run-static.sh", 0);
    read(fd, buf, 6000);

    // run tests
    for (int row = 0; row < PROG_NUM; row++)
    {
        read_test_name();
        int npid = fork();
        assert(npid >= 0);
        int child_return;
        if (npid == 0)
        {
            // child
            execve("./runtest.exe", argv, NULL);
        }
        else
        {
            // parent
            child_return = -1;
            waitpid(npid, &child_return, 0);
        }
    }
    return 0;
}