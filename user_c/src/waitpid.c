#include "stdio.h"
#include "stdlib.h"
#include "unistd.h"

int i = 1000;
int test_waitpid(void){
    int flag = 0;
    TEST_START(__func__);
    int cpid, wstatus;
    cpid = fork();
    assert(cpid != -1);
    if(cpid == 0){
	while(i--);
	sched_yield();
	printf("This is child process\n");
        exit(3);
    }else{
	pid_t ret = waitpid(cpid, &wstatus, 0);
	assert(ret != -1);
	if(ret == cpid && WEXITSTATUS(wstatus) == 3)
	    printf("waitpid successfully.\nwstatus: %x\n", WEXITSTATUS(wstatus));
	else{
        printf("ret: %d\ncpid: %d\nwstatus: %d\nWEXITSTATUS(wstatus): %x\n", ret, cpid, wstatus, WEXITSTATUS(wstatus));
	    printf("waitpid error.\n");
        flag = -1;
    }
    TEST_END(__func__);
    return flag;
    }
}

int main(void){
    return test_waitpid();
}
