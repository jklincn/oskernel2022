#include <stdio.h>
#include <unistd.h>
#include <stdlib.h>

/*
理想结果：得到进程 pid，注意要关注 pid 是否符合内核逻辑，不要单纯以 Test OK! 作为判断。
*/

int test_getpid(){
    TEST_START(__func__);
    int pid = getpid();
    if(pid < 0) return pid;
    printf("getpid success.\npid = %d\n", pid);
    TEST_END(__func__);
    return 0;
}

int main(void) {
	return test_getpid();
}
