# rCore
## 对接比赛测试程序进度
### 比赛系统调用接口（C）
| 完成 | 系统调用          | 系统调用号 |
| ---- | ----------------- | ---------- |
|      | SYSCALL_DUP       | 24         |
|      | SYSCALL_OPEN      | 56         |
|      | SYSCALL_CLOSE     | 57         |
|      | SYSCALL_PIPE      | 59         |
|      | SYSCALL_READ      | 63         |
|      | SYSCALL_WRITE     | 64         |
| ✔    | SYSCALL_EXIT      | 93         |
| ✔    | SYSCALL_NANOSLEEP | 101        |
| ✔    | SYSCALL_YIELD     | 124        |
|      | SYSCALL_KILL      | 129        |
| ✔    | SYSCALL_UNAME     | 160        |
| ✔    | SYSCALL_GET_TIME  | 169        |
| ✔    | SYSCALL_GETPID    | 172        |
|      | SYSCALL_GETPPID   | 173        |
| ✔    | SYSCALL_FORK      | 220        |
| ✔    | SYSCALL_EXEC      | 221        |
| ✔    | SYSCALL_WAITPID   | 260        |

### 比赛测试程序
| 完成 | 测试用例   | 简单描述 |
| ---- | ---------- | -------- |
|      | brk.c      |
|      | fstat.c    |
|      | getcwd.c   |
|      | getdents.c |
|      | mmap.c     |
|      | munmap.c   |

#### I/O相关
| 完成 | 测试用例 | 简单描述 |
| ---- | -------- | -------- |
| 🛠    | dup.c    |
| 🛠    | dup2.c   |
| 🛠    | pipe.c   |

#### 进程相关
| 完成 | 测试用例  | 简单描述                      |
| ---- | --------- | ----------------------------- |
| ✔    | fork.c    | SYSCALL_FORK、SYSCALL_WAITPID |
| 🛠    | clone.c   |
| ✔    | execve.c  | SYSCALL_EXEC                  |
| ✔    | exit.c    | SYSCALL_EXIT                  |
| ✔    | getpid.c  | SYSCALL_GETPID                |
|      | getppid.c |
| ✔    | sleep.c   |
| 🛠    | yield.c   |
| 🛠    | wait.c    |
| 🛠    | waitpid.c |

#### FAT32相关
| 完成 | 测试用例 | 简单描述 |
| ---- | -------- | -------- |
|      | open.c   |
|      | openat.c |
|      | close.c  |
|      | read.c   |
|      | write.c  |
|      | mount.c  |
|      | umount.c |
|      | mkdir.c  |
|      | chdir.c  |
|      | unlink.c |

#### 系统信息
| 完成 | 测试用例       | 简单描述      |
| ---- | -------------- | ------------- |
| ✔    | uname.c        | SYSCALL_UNAME |
| 🛠    | times.c        |
| 🛠    | gettimeofday.c |