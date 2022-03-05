# user/build.py
# 用于构建应用程序linker.ld,保证应用内存空间不相交

import os

base_address = 0x80400000
step = 0x20000
linker = 'src/linker.ld'

app_id = 0
apps = os.listdir('src/bin')
apps.sort()
for app in apps:
    app = app[:app.find('.')]
    lines = []
    lines_before = []
    with open(linker, 'r') as f:
        # 将linker.ld中的基地址修改为每个程序的不同的基地址
        # 并将原来的数据保存到lines_before
        for line in f.readlines():
            lines_before.append(line)
            line = line.replace(hex(base_address), hex(base_address+step*app_id))
            lines.append(line)
    with open(linker, 'w+') as f:
        f.writelines(lines)
    # 每次修改linker.ld文件后编译对应应用程序
    os.system('cargo build --bin %s --release' % app)
    print('[build.py] application %s start with address %s' %(app, hex(base_address+step*app_id)))
    # 恢复原来的linker.ld文件
    with open(linker, 'w+') as f:
        f.writelines(lines_before)
    app_id = app_id + 1
