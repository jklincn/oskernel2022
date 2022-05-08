#include "stdio.h"
#include "stdlib.h"
#include "unistd.h"

/*
 * 测试成功则输出：
 * "  I am test_echo."
 * 测试失败则输出：
 * "  execve error."
 */
int test_exec(void) {
    TEST_START(__func__);
    exec("test_echo");
    printf("  execve error.\n");
    return -1;
}

int main(void) {
    return test_exec();
    
}
