#include "stdio.h"
#include "unistd.h"
#include "stdlib.h"

char *argv_busybox[] = {"./busybox", "sh","busybox_testcode.sh", 0};
char *argv_lua[] = {"./busybox", "sh","lua_testcode.sh", 0};

int main()
{
    printf("[TEST] start busybox test!\n");
    
    int npid = fork();
    assert(npid >= 0);
    int child_return;
    if (npid == 0)
    {
        execve("./busybox", argv_busybox, NULL);
    }
    else
    {
        // parent
        child_return = -1;
        waitpid(npid, &child_return, 0);
    }

    printf("[TEST] start lua test!\n");
    
    npid = fork();
    assert(npid >= 0);
    if (npid == 0)
    {
        execve("./busybox", argv_lua, NULL);
    }
    else
    {
        // parent
        child_return = -1;
        waitpid(npid, &child_return, 0);
    }

    printf("[TEST] test finish!\n");
    return 0;

    
}