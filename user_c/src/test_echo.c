#include "stdio.h"

/*
 * for execve
 */

int main(int argc, char* argv[]) {
    printf("  I am test_echo.\nexecve success.\n");
    TEST_END("test_execve");
    return 0;
}