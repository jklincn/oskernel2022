#include "stdio.h"
#include "unistd.h"
#include "stdlib.h"

// 测试所有程序
#define TEST_ALL

#ifndef TEST_ALL
char *argv[] = {"./runtest.exe", "-w", "entry-dynamic.exe", "prog_name", 0};
#else
int offset = 0;
#define PROG_NAME_MAX_LENGTH 40
#define BUFFER_SIZE 6500

#define STATIC_PROG_NUM 109
#define DYNAMIC_PROG_NUM 111

char prog_name[PROG_NAME_MAX_LENGTH];
char buf[BUFFER_SIZE];

char *static_argv[] = {"./runtest.exe", "-w", "entry-static.exe", prog_name, 0};
char *dynamic_argv[] = {"./runtest.exe", "-w", "entry-dynamic.exe", prog_name, 0};

// 静态跳过程序
#define STATIC_PROG_PASS_LENGTH 20
char *static_prog_pass[] = {
                     "pthread_cancel_points",
                     "pthread_cancel",
                     "pthread_cond",
                     "pthread_tsd",
                     "fflush_exit",
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

// 动态跳过程序
#define DYNAMIC_PROG_PASS_LENGTH 24
char *dynamic_prog_pass[] = {
                     "pthread_cancel_points",
                     "pthread_cancel",
                     "pthread_cond",
                     "pthread_tsd",
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
                     "stat",
                     "tls_init",
                     "tls_local_exec",
                     "ungetc",
                     "utime",
                     "lseek_large",
                     "setvbuf_unget",
                     "tls_get_new_dtv",
                     };

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

int ifpass_static()
{
    for (int i = 0; i < STATIC_PROG_PASS_LENGTH; i++)
        if (!mystrcmp(static_prog_pass[i], prog_name))
            return 1;
    return 0;
}

int ifpass_dynamic()
{
    for (int i = 0; i < DYNAMIC_PROG_PASS_LENGTH; i++)
        if (!mystrcmp(dynamic_prog_pass[i], prog_name))
            return 1;
    return 0;
}
#endif

int main()
{
#ifndef TEST_ALL
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
#else
    // 运行静态测试程序
    int fd = open("./run-static.sh", 0);
    read(fd, buf, BUFFER_SIZE);

    for (int row = 0; row < STATIC_PROG_NUM; row++)
    {
        read_test_name();

        // pass some programs
        if (ifpass_static())
            continue;

        int npid = fork();
        assert(npid >= 0);
        int child_return;
        if (npid == 0)
        {
            // child
            execve("./runtest.exe", static_argv, NULL);
        }
        else
        {
            // parent
            child_return = -1;
            waitpid(npid, &child_return, 0);
        }
    }

    // 运行动态测试程序
    fd = open("./run-dynamic.sh", 0);
    offset = 0;
    read(fd, buf, BUFFER_SIZE);

    for (int row = 0; row < DYNAMIC_PROG_NUM; row++)
    {
        read_test_name();

        // pass some programs
        if (ifpass_dynamic())
            continue;

        int npid = fork();
        assert(npid >= 0);
        int child_return;
        if (npid == 0)
        {
            // child
            execve("./runtest.exe", dynamic_argv, NULL);
        }
        else
        {
            // parent
            child_return = -1;
            waitpid(npid, &child_return, 0);
        }
    }
    return 0;
#endif
}