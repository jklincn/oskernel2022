#include "stdio.h"
#include "stdlib.h"
#include "unistd.h"

/*
 * 测试成功则输出：
 * "exit OK."
 * 测试失败则输出：
 * "exit ERR."
 */
void test_exit(void) {
    TEST_START(__func__);
    int cpid, waitret, wstatus;
    cpid = fork();
    assert(cpid != -1);
    if (cpid == 0) {
        exit(0);
    }
    else {
        // while(1){       
        //     waitret = wait(&wstatus);
        //     if (waitret >= 0)break;
        // }
        /*----------SYSCALL_WAIT需修改以支持一下代码-----------*/
        waitret = wait(&wstatus);
        printf("waitret:%d cpid:%d\n", waitret,cpid);
        if (waitret == cpid) printf("exit OK.\n");
        else printf("exit ERR.\n");
    }
    TEST_END(__func__);
}

int main(void) {
    test_exit();
    return 0;
}
