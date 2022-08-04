#include "stdio.h"
#include "unistd.h"
#include "stdlib.h"



// char *argv[] = {"./runtest.exe", "-w", "entry-dynamic.exe", "lseek_large", 0};

// int offset = 0;
// #define PROG_NAME_MAX_LENGTH 40
// #define BUFFER_SIZE 6500

// char prog_name[PROG_NAME_MAX_LENGTH];
// char buf[BUFFER_SIZE];

// char *static_argv[] = {"./runtest.exe", "-w", "entry-static.exe", prog_name, 0};
// char *dynamic_argv[] = {"./runtest.exe", "-w", "entry-dynamic.exe", prog_name, 0};

// void read_test_name()
// {
//     for (int i = 0; i < PROG_NAME_MAX_LENGTH; i++)
//         *(prog_name + i) = '\0';
//     // skip space
//     for (int k = 0; k < 3; k++)
//     {
//         for (; buf[offset] != ' '; offset++)
//             ;
//         offset++;
//     }
//     int k;
//     for (k = 0; buf[offset] != '\n'; k++, offset++)
//         *(prog_name + k) = buf[offset];
//     offset++;
// }

// int mystrcmp(const char *s1, const char *s2)
// {
//     while (*s1 && *s2 && *s1 == *s2)
//     {
//         s1++;
//         s2++;
//     }
//     return *s1 - *s2;
// }

// int ifpass_static()
// {
//     for (int i = 0; i < STATIC_PROG_PASS_LENGTH; i++)
//         if (!mystrcmp(static_prog_pass[i], prog_name))
//             return 1;
//     return 0;
// }


int main()
{

    //test only one program
    int npid = fork();
    assert(npid >= 0);
    int child_return;
    if (npid == 0)
    {
        execve("busybox", NULL, NULL);
    }
    else
    {
        // parent
        child_return = -1;
        waitpid(npid, &child_return, 0);
    }

    return 0;

    // // 运行静态测试程序
    // int fd = open("./run-static.sh", 0);
    // read(fd, buf, BUFFER_SIZE);

    // for (int row = 0; row < STATIC_PROG_NUM; row++)
    // {
    //     read_test_name();

    //     // pass some programs
    //     if (ifpass_static())
    //         continue;

    //     int npid = fork();
    //     assert(npid >= 0);
    //     int child_return;
    //     if (npid == 0)
    //     {
    //         // child
    //         execve("./runtest.exe", static_argv, NULL);
    //     }
    //     else
    //     {
    //         // parent
    //         child_return = -1;
    //         waitpid(npid, &child_return, 0);
    //     }
    // }

}