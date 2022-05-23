#include "stdio.h"
#include "unistd.h"
#include "stdlib.h"

/*
 * 能通过测试则输出：
 * "  getppid success. ppid : [num]"
 * 不能通过测试则输出：
 * "  getppid error."
 */

int test_getppid()
{
    TEST_START(__func__);
    pid_t ppid = getppid();
    if(ppid > 0) printf("  getppid success. ppid : %d\n", ppid);
    else printf("  getppid error. ppid : %d\n", ppid);
    TEST_END(__func__);
    return ppid > 0 ? 0 : ppid;
}

int main(void) {
	return test_getppid();
}
