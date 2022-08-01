#include "stdio.h"
#include "unistd.h"
#include "stdlib.h"

#define PROG_NUM 110
#define PROG_NAME_MAX_LENGTH 40

char prog_name[PROG_NAME_MAX_LENGTH];
char buf[6000];

// #define DYNAMIC
#define TEST_ONE

#ifndef DYNAMIC
    char *argv[] = {"./runtest.exe", "-w", "entry-static.exe", "daemon_failure", 0};
#else
    char *argv[] = {"./runtest.exe", "-w", "entry-dynamic.exe", prog_name, 0};
#endif

int offset = 0;

#define PROG_PASS_LENGTH 13
char *prog_pass[] = {
                     "pthread_cancel_points",
                     "pthread_cancel",
                     "pthread_cond",
                     "pthread_tsd",
                     "daemon_failure",
                     "fflush_exit",
                     "pthread_robust_detach",
                     "pthread_cancel_sem_wait",
                     "pthread_cond_smasher",
                     "pthread_condattr_setclock",
                     "pthread_exit_cancel",
                     "pthread_once_deadlock",
                     "pthread_rwlock_ebusy",
                     };

void read_test_name()
{
    for (int i = 0; i < 40; i++)
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

int main()
{
    int fd = open("./run-static.sh", 0);
    read(fd, buf, 6000);


#ifndef TEST_ONE
    // run all tests
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