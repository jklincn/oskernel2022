# RongOS

RongOS 是使用 Rust 语言编写的基于 RISC-V64 的微型操作系统，实现了进程控制、内存管理、文件系统这三大操作系统基础模块，且支持在 QEMU 虚拟环境和 K210 硬件平台上运行。

RongOS 获得2022全国大学生计算机系统能力大赛操作系统设计大赛内核实现赛道全国二等奖（位列全国第9名）。[获奖结果](https://compiler.educg.net/#/chDetail)

# 项目特征

- 使用 Rust 语言编写
- 支持 79 条系统调用
- 独立的应用地址空间和内核地址空间
- 适配 SV39 多级页表、使用 Lazy、COW 机制
- 支持虚拟文件系统，适配自制的 FAT32 文件系统
- 支持 ELF 格式的静态/动态链接程序
- 通过 lmbench 基准测试，支持 busybox、lua 等程序
- 详细的项目文档和 doc comment 格式代码注释

详细文档：[RongOS设计与实现文档.pdf](https://github.com/jklincn/oskernel2022/blob/master/doc/RongOS%E8%AE%BE%E8%AE%A1%E4%B8%8E%E5%AE%9E%E7%8E%B0%E6%96%87%E6%A1%A3.pdf)
