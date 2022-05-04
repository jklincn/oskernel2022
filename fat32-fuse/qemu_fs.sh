# 制作一个全0的3G的镜像文件
dd if=/dev/zero of=fat32.img bs=3M count=100

# 格式化为 fat32
sudo mkfs.vfat -F 32 fat32.img
sudo chmod 777 fat32.img
#sudo mount fat32.img sd_mnt
#sudo chmod 777 sd_mnt