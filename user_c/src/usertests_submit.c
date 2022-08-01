#include "stdio.h"
#include "unistd.h"
#include "stdlib.h"

// 测试动态链接程序
// #define DYNAMIC

// 测试单一程序
// #define TEST_ONE

#define PROG_NAME_MAX_LENGTH 40
#define BUFFER_SIZE 6500

#ifndef DYNAMIC
#define PROG_NUM 109
#else
#define PROG_NUM 111
#endif

char prog_name[PROG_NAME_MAX_LENGTH];
char buf[BUFFER_SIZE];

// 设置调试程序，prog_name 只有在测试全部程序下使用
#ifndef DYNAMIC
    char *argv[] = {"./runtest.exe", "-w", "entry-static.exe", prog_name, 0};
#else
    char *argv[] = {"./runtest.exe", "-w", "entry-dynamic.exe", prog_name, 0};
#endif

#ifndef TEST_ONE
int offset = 0;
#ifndef DYNAMIC
// 静态跳过程序
#define PROG_PASS_LENGTH 21
char *prog_pass[] = {
                     "pthread_cancel_points",
                     "pthread_cancel",
                     "pthread_cond",
                     "pthread_tsd",
                     "fflush_exit",
                     "daemon_failure",
                     "pthread_robust_detach",
                     "pthread_cancel_sem_wait",
                     "pthread_cond_smasher",
                     "pthread_condattr_setclock",
                     "pthread_exit_cancel",
                     "pthread_once_deadlock",
                     "pthread_rwlock_ebusy",
                     //
                     "fscanf",
                     "fwscanf",
                     "sscanf_long",
                     "stat",
                     "ungetc",
                     "utime",
                     "lseek_large",
                     "setvbuf_unget",
                     };
#else
// 动态跳过程序
#define PROG_PASS_LENGTH 17
char *prog_pass[] = {
                     "pthread_cancel_points",
                     "pthread_cancel",
                     "pthread_cond",
                     "pthread_tsd",
                     "daemon_failure",
                     "fflush_exit",
                     "pthread_robust_detach",
                     "pthread_cond_smasher",
                     "pthread_condattr_setclock",
                     "pthread_exit_cancel",
                     "pthread_once_deadlock",
                     "pthread_rwlock_ebusy",
                     //
                     "fscanf",
                     "fwscanf",
                     "sem_init",
                     "socket",
                     "sscanf_long",

                     };
#endif

void read_test_name()
{
    for (int i = 0; i < PROG_NAME_MAX_LENGTH; i++)
        *(prog_name + i) = '\0';
    // skip space
    for (int k = 0; k < 3; k++)
    {
        for (; buf[offset] != ' '; offset++)
            ;
        offset++;
    }
    int k;
    for (k = 0; buf[offset] != '\n'; k++, offset++)
        *(prog_name + k) = buf[offset];
    offset++;
}

int mystrcmp(const char *s1, const char *s2)
{
    while (*s1 && *s2 && *s1 == *s2)
    {
        s1++;
        s2++;
    }
    return *s1 - *s2;
}

int ifpass()
{
    for (int i = 0; i < PROG_PASS_LENGTH; i++)
        if (!mystrcmp(prog_pass[i], prog_name))
            return 1;
    return 0;
}
#endif

int main()
{
#ifndef TEST_ONE
    #ifndef DYNAMIC
    // run all tests
    int fd = open("./run-static.sh", 0);
    #else
    int fd = open("./run-dynamic.sh", 0);
    #endif
    read(fd, buf, BUFFER_SIZE);

    for (int row = 0; row < PROG_NUM; row++)
    {
        read_test_name();

        // pass some programs
        if (ifpass())
            continue;

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
#else
    //test only one program
    int npid = fork();
    assert(npid >= 0);
    int child_return;
    if (npid == 0)
    {
        execve("./runtest.exe", argv, NULL);
    }
    else
    {
        // parent
        child_return = -1;
        waitpid(npid, &child_return, 0);
    }

    return 0;
#endif
}