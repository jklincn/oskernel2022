#include "stdio.h"
#include "stdlib.h"
#include "unistd.h"

/*
 * 测试成功则输出：
 * "exit OK."
 * 测试失败则输出：
 * "exit ERR."
 */
int test_exit(void) {
    TEST_START(__func__);
    int cpid, waitret, wstatus;
    cpid = fork();
    assert(cpid != -1);
    if (cpid == 0) {
        exit(98988);
    }
    else {
        waitret = wait(&wstatus);
        printf("waitret:%d cpid:%d child_retrun:%d\n", waitret, cpid, wstatus);
        if (waitret == cpid && wstatus == 98988) {
            printf("exit OK.\n");
            return 0;
        }
        else {
            printf("exit ERR.\n");
            return -1;
        }
    }
    TEST_END(__func__);
}

int main(void) {
    return test_exit();
    
}
