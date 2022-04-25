# rCore
- [rCore](#rcore)
  - [对接比赛测试程序进度](#对接比赛测试程序进度)
    - [比赛系统调用接口（C）](#比赛系统调用接口c)

## 对接比赛测试程序进度
### 比赛系统调用接口（C）
| 完成 | 系统调用         | 系统调用号 |
| ---- | ---------------- | ---------- |
| ✔    | SYSCALL_OPEN     | 56         |
|      | SYSCALL_DUP      | 24         |
|      | SYSCALL_CLOSE    | 57         |
|      | SYSCALL_PIPE     | 59         |
|      | SYSCALL_READ     | 63         |
|      | SYSCALL_WRITE    | 64         |
| ✔    | SYSCALL_EXIT     | 93         |
| ✔    | SYSCALL_YIELD    | 124        |
|      | SYSCALL_KILL     | 129        |
| ✔    | SYSCALL_UNAME    | 160        |
|      | SYSCALL_GET_TIME | 169        |
|      | SYSCALL_GETPID   | 172        |
| ✔    | SYSCALL_FORK     | 220        |
| ✔    | SYSCALL_EXEC     | 221        |
| ✔    | SYSCALL_WAITPID  | 260        |