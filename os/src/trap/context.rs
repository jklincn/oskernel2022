use riscv::register::sstatus::{self, Sstatus, SPP};

#[repr(C)]
pub struct TrapContext {
    /// 通用寄存器`x[0]~x[31]`
    pub x: [usize; 32],
    /// 提供状态信息
    pub sstatus: Sstatus,
    /// 记录 Trap 发生之前执行的最后一条指令的地址
    pub sepc: usize,
    // 一下数据在应用初始化的时候由内核写入应用地址空间中的TrapContext中的相应位置，此后不再修改
    /// 内核地址空间的 token ，即内核页表的起始物理地址
    pub kernel_satp: usize,
    /// 当前应用在内核地址空间中的内核栈栈顶的虚拟地址
    pub kernel_sp: usize,
    /// 内核中 trap handler 入口点的虚拟地址
    pub trap_handler: usize,
}

impl TrapContext {
    pub fn set_sp(&mut self, sp: usize) {
        self.x[2] = sp;
    }
    pub fn app_init_context(
        entry: usize,
        sp: usize,
        kernel_satp: usize,
        kernel_sp: usize,
        trap_handler: usize,
    ) -> Self {
        let mut sstatus = sstatus::read();
        sstatus.set_spp(SPP::User);
        let mut cx = Self {
            x: [0; 32],
            sstatus,
            sepc: entry,    // Trap 返回后到程序入口
            kernel_satp,
            kernel_sp,
            trap_handler,
        };
        cx.set_sp(sp);
        cx
    }

}

impl TrapContext {
    #[allow(unused)]
    pub fn debug_show(&self){
        println!("------------------TrapContext info------------------");
        println!("sepc:         0x{:x}",self.sepc);
        println!("kernel_satp:  0x{:x}",self.kernel_satp);
        println!("kernel_sp:    0x{:x}",self.kernel_sp);
        println!("trap_handler: 0x{:x}",self.trap_handler);
        println!("zero: 0x{:x}", self.x[0]);
        println!("ra: 0x{:x}", self.x[1]);
        println!("sp: 0x{:x}", self.x[2]);
        println!("gp: 0x{:x}", self.x[3]);
        println!("tp: 0x{:x}", self.x[4]);
        println!("t0: 0x{:x}", self.x[5]);
        println!("t1: 0x{:x}", self.x[6]);
        println!("t2: 0x{:x}", self.x[7]);
        println!("s0/fp: 0x{:x}", self.x[8]);
        println!("s1: 0x{:x}", self.x[9]);
        println!("a0: 0x{:x}", self.x[10]);
        println!("a1: 0x{:x}", self.x[11]);
        println!("a2: 0x{:x}", self.x[12]);
        println!("a3: 0x{:x}", self.x[13]);
        println!("a4: 0x{:x}", self.x[14]);
        println!("a5: 0x{:x}", self.x[15]);
        println!("a6: 0x{:x}", self.x[16]);
        println!("a7: 0x{:x}", self.x[17]);
        println!("s2: 0x{:x}", self.x[18]);
        println!("s3: 0x{:x}", self.x[19]);
        println!("s4: 0x{:x}", self.x[20]);
        println!("s5: 0x{:x}", self.x[21]);
        println!("s6: 0x{:x}", self.x[22]);
        println!("s7: 0x{:x}", self.x[23]);
        println!("s8: 0x{:x}", self.x[24]);
        println!("s9: 0x{:x}", self.x[25]);
        println!("s10: 0x{:x}", self.x[26]);
        println!("s11: 0x{:x}", self.x[27]);
        println!("t3: 0x{:x}", self.x[28]);
        println!("t4: 0x{:x}", self.x[29]);
        println!("t5: 0x{:x}", self.x[30]);
        println!("t6: 0x{:x}", self.x[31]);
        println!("----------------------------------------------------");
    }
    
}
