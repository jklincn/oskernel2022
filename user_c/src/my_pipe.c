#include "stdio.h"
#include "stdlib.h"
#include "unistd.h"
#include "string.h"
/*
 * 成功测试时的输出：
 * "  Write to pipe successfully."
 */
static int fd[2];

void test_pipe(void)
{
    TEST_START(__func__);
    int cpid;
    char buf[128] = {0};
    int ret = pipe(fd);
    assert(ret != -1);
    const char *data = "  Write to pipe successfully.\n";
    cpid = fork();
    printf("cpid: %d\n", cpid);
    if (cpid > 0)
    {
        printf("cpid: %d close enter\n", cpid);
        close(fd[1]);
        printf("cpid: %d close 1\n", cpid);
        while (read(fd[0], buf, 1) > 0)
        {
            printf(">0\n");
            write(STDOUT, buf, 1);
        }

        printf("cpid: %d write buf\n", cpid);
        write(STDOUT, "\n", 1);
        printf("cpid: %d write \\n\n", cpid);
        close(fd[0]);
        wait(NULL);
    }
    else
    {
        printf("cpid: %d close enter\n", cpid);
        close(fd[0]);
        printf("cpid: %d close 0\n", cpid);
        write(fd[1], data, strlen(data));
        printf("cpid: %d write\n", cpid);
        close(fd[1]);
        printf("cpid: %d close 1\n", cpid);
        exit(0);
    }
    TEST_END(__func__);
}

int main(void)
{
    test_pipe();
    return 0;
}
