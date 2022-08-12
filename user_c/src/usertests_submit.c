#include "stdio.h"
#include "unistd.h"
#include "stdlib.h"

// #define TEST

char *argv_sh[] = {"./busybox", "sh", 0};
char *argv_busybox[] = {"./busybox", "sh", "busybox_testcode.sh", 0};
char *argv_lua[] = {"./busybox", "sh", "lua_testcode.sh", 0};

int main()
{
#ifndef TEST
    int npid = fork();
    assert(npid >= 0);
    int child_return;
    if (npid == 0)
        execve("./busybox", argv_sh, NULL);
    else
    {
        child_return = -1;
        waitpid(npid, &child_return, 0);
    }
#else
    printf("[TEST] start busybox test!\n");

    int npid = fork();
    assert(npid >= 0);
    int child_return;
    if (npid == 0)
        execve("./busybox", argv_busybox, NULL);
    else
    {
        child_return = -1;
        waitpid(npid, &child_return, 0);
    }

    printf("[TEST] start lua test!\n");

    npid = fork();
    assert(npid >= 0);
    if (npid == 0)
        execve("./busybox", argv_lua, NULL);
    else
    {
        child_return = -1;
        waitpid(npid, &child_return, 0);
    }

    printf("[TEST] test finish!\n");
#endif
    return 0;

}