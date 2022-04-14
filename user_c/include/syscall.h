#define FD_STDIN 0
#define FD_STDOUT 1

#define SYSCALL_WRITE 64
#define SYSCALL_EXIT 93

typedef unsigned long long usize;
typedef long long isize;

isize syscall(usize id, usize a0, usize a1, usize a2);

isize sys_write(usize fd, char* buf, usize len);
isize sys_exit(int exit_code);