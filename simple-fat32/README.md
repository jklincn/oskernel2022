simple-fat32 中移入了输出模块，可以在调试中使用 println! 宏进行打印（还实现了多种信息类别）。

create_img.sh 会创建一个 FAT32 格式的镜像，用于 qemu 中进行测试。

在 os 的 Makefile 中，copy_to_fs 会自动调用 create_img.sh 并将 user_c 下的自定义测试程序拷贝到镜像中。