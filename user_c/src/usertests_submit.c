#include "stdio.h"
#include "unistd.h"
#include "stdlib.h"
#include "stddef.h"

// #define SHELL
#define TEST
#define LMBENCH

char *argv_sh[] = {"./busybox", "sh", 0};
char *argv_busybox[] = {"./busybox", "sh", "busybox_testcode.sh", 0};
char *argv_lua[] = {"./busybox", "sh", "lua_testcode.sh", 0};

#ifdef LMBENCH
char *argv_lmbench0[] = {"./lmbench_all", "lat_syscall", "-P", "1", "null", 0};
char *argv_lmbench1[] = {"./lmbench_all", "lat_syscall", "-P", "1", "read", 0};
char *argv_lmbench2[] = {"./lmbench_all", "lat_syscall", "-P", "1", "write", 0};
char *argv_lmbench3[] = {"./lmbench_all", "lat_syscall", "-P", "1", "stat", "/var/tmp/lmbench", 0};
char *argv_lmbench4[] = {"./lmbench_all", "lat_syscall", "-P", "1", "fstat", "/var/tmp/lmbench", 0};
char *argv_lmbench5[] = {"./lmbench_all", "lat_syscall", "-P", "1", "open", "/var/tmp/lmbench", 0};
char *argv_lmbench6[] = {"./lmbench_all", "lat_select", "-n", "100", "-P", "1", "file", 0};
char *argv_lmbench7[] = {"./lmbench_all", "lat_sig", "-P", "1", "install", 0};
char *argv_lmbench8[] = {"./lmbench_all", "lat_sig", "-P", "1", "catch", 0};
char *argv_lmbench9[] = {"./lmbench_all", "lat_sig", "-P", "1", "prot", "lat_sig", 0};
char *argv_lmbench10[] = {"./lmbench_all", "lat_pipe", "-P", "1", 0};
char *argv_lmbench11[] = {"./lmbench_all", "lat_proc", "-P", "1", "fork", 0};
char *argv_lmbench12[] = {"./lmbench_all", "lat_proc", "-P", "1", "exec", 0};
char *argv_lmbench13[] = {"./lmbench_all", "lat_proc", "-P", "1", "shell", 0};
char *argv_lmbench14[] = {"./lmbench_all", "lmdd", "label=\"File /var/tmp/XXX write bandwidth:\"", "of=/var/tmp/XXX", "move=1m", "fsync=1", "print=3", 0};
char *argv_lmbench15[] = {"./lmbench_all", "lat_pagefault", "-P", "1", "/var/tmp/XXX", 0};
char *argv_lmbench16[] = {"./lmbench_all", "lat_mmap", "-P", "1", "512k", "/var/tmp/XXX", 0};
char *argv_lmbench17[] = {"./lmbench_all", "lat_fs", "/var/tmp", 0};
char *argv_lmbench18[] = {"./lmbench_all", "bw_pipe", "-P", "1", 0};
char *argv_lmbench19[] = {"./lmbench_all", "bw_file_rd", "-P", "1", "512k", "io_only", "/var/tmp/XXX", 0};
char *argv_lmbench20[] = {"./lmbench_all", "bw_file_rd", "-P", "1", "512k", "open2close", "/var/tmp/XXX", 0};
char *argv_lmbench21[] = {"./lmbench_all", "bw_mmap_rd", "-P", "1", "512k", "mmap_only", "/var/tmp/XXX", 0};
char *argv_lmbench22[] = {"./lmbench_all", "bw_mmap_rd", "-P", "1", "512k", "open2close", "/var/tmp/XXX", 0};
char *argv_lmbench23[] = {"./lmbench_all", "lat_ctx", "-P", "1", "-s", "32", "2", 0};
char *argv_lmbench24[] = {"./lmbench_all", "lat_ctx", "-P", "1", "-s", "32", "4", 0};
char *argv_lmbench25[] = {"./lmbench_all", "lat_ctx", "-P", "1", "-s", "32", "8", 0};
char *argv_lmbench26[] = {"./lmbench_all", "lat_ctx", "-P", "1", "-s", "32", "16", 0};
char *argv_lmbench27[] = {"./lmbench_all", "lat_ctx", "-P", "1", "-s", "32", "24", 0};
char *argv_lmbench28[] = {"./lmbench_all", "lat_ctx", "-P", "1", "-s", "32", "32", 0};
char *argv_lmbench29[] = {"./lmbench_all", "lat_ctx", "-P", "1", "-s", "32", "64", 0};
char *argv_lmbench30[] = {"./lmbench_all", "lat_ctx", "-P", "1", "-s", "32", "96", 0};
const int lmbench_test_num = 20;
char **argv_lmbench[] =
    {
        argv_lmbench0,
        argv_lmbench1,
        argv_lmbench2,
        argv_lmbench3,
        argv_lmbench4,
        argv_lmbench5,
        argv_lmbench6,
        argv_lmbench7,
        argv_lmbench8,
        // argv_lmbench9,
        argv_lmbench10,
        argv_lmbench11,
        argv_lmbench12,
        // argv_lmbench13,
        argv_lmbench14,
        argv_lmbench15,
        argv_lmbench16,
        argv_lmbench17,
        // argv_lmbench18,
        argv_lmbench19,
        argv_lmbench20,
        argv_lmbench21,
        argv_lmbench22,
        // argv_lmbench23,
        // argv_lmbench24,
        // argv_lmbench25,
        // argv_lmbench26,
        // argv_lmbench27,
        // argv_lmbench28,
        // argv_lmbench29,
        // argv_lmbench30,
};
#endif

int main()
{
    int npid = 0, child_return = 0;
    TimeVal start_tv, end_tv;
    sys_get_time(&start_tv, 0);

#ifdef SHELL
    npid = fork();
    assert(npid >= 0);
    if (npid == 0)
        execve("./busybox", argv_sh, NULL);
    else
    {
        child_return = -1;
        waitpid(npid, &child_return, 0);
    }
    return 0;
#endif
#ifdef TEST
    printf("[TEST] start busybox test!\n");

    npid = fork();
    assert(npid >= 0);
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
#endif

#ifdef LMBENCH
    printf("[TEST] start lmbench test!\n");

    for (int i = 0; i < lmbench_test_num; i++)
    {
        npid = fork();
        assert(npid >= 0);
        if (npid == 0)
            execve("./lmbench_all", argv_lmbench[i], NULL);
        else
        {
            child_return = -1;
            waitpid(npid, &child_return, 0);
        }
    }
#endif
    sys_get_time(&end_tv, 0);
    printf("[TEST] spend time: %ds %dus\n", end_tv.sec - start_tv.sec, end_tv.usec - start_tv.usec);
    printf("[TEST] test finish!\n");
    return 0;
}