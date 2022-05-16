#include "unistd.h"
#include "stdio.h"
#include "stdlib.h"

/*
 * 测试通过时应输出：
 * "  new fd is 3."
 */

int test_dup(){
	TEST_START(__func__);
	int fd = dup(STDOUT);
	assert(fd >=0);
	printf("  new fd is %d.\n", fd);
	TEST_END(__func__);
	return fd == 3 ? 0 : -1;
}

int main(void) {
	return test_dup();
}
