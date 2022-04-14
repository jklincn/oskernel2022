#include "syscall.h"

int main(){
    char* s = "hello world in c\n";
    sys_write(FD_STDOUT, s, 18);
    return 0;
}