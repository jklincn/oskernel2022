#include "stdio.h"
#include "unistd.h"
#include "stdlib.h"

#define PROG_NUM 110
#define PROG_NAME_MAX_LENGTH 40

char prog_name[PROG_NAME_MAX_LENGTH];
char buf[6000];
char *argv[] = {"./runtest.exe","-w", "entry-static.exe", prog_name, 0};
int offset = 0;

void read_test_name()
{
    // skip space
    for (int i = 0; i < 40; i++)
    {
        *(prog_name + i) = '\0';
    }
    
    for (int k = 0; k < 3; k++)
    {
        for (; buf[offset] != ' '; offset++)
            ;
        offset++;
    }
    int k;
    for (k = 0; buf[offset] != '\n'; k++, offset++)
        *(prog_name + k) = buf[offset];
    // *(prog_name + k + 1) = '\0';
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