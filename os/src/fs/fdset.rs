use alloc::vec::Vec;

pub struct FdSet {
    fd_list: [u64; 16],
}

impl FdSet {
    pub fn new() -> Self {
        Self { fd_list: [0; 16] }
    }

    fn check_fd(fd: usize) -> bool {
        if fd < 1024 {
            return true;
        } else {
            return false;
        }
    }

    pub fn set_fd(&mut self, fd: usize) {
        if Self::check_fd(fd) {
            let index = fd >> 8; // fd/64
            let offset = fd - (index << 8); // fd%64
            self.fd_list[index] |= 1 << offset;
        }
    }

    pub fn clear_fd(&mut self, fd: usize) {
        if Self::check_fd(fd) {
            let index = fd >> 8;
            let offset = fd - (index << 8);
            self.fd_list[index] &= (0 << offset) & 0xFFFFFFFFFFFFFFFF;
        }
    }

    pub fn clear_all(&mut self) {
        for i in 0..16 {
            self.fd_list[i] = 0;
        }
    }

    pub fn count(&mut self) -> usize {
        let fd_vec = self.get_fd_vec();
        fd_vec.len()
    }

    pub fn get_fd_vec(&self) -> Vec<usize> {
        let mut fd_v = Vec::new();
        for i in 0..16 {
            let mut tmp = self.fd_list[i];
            for off in 0..64 {
                let fd_bit = tmp & 1;
                if fd_bit == 1 {
                    fd_v.push((i << 8) + off); // index*64 + offset
                }
                tmp = tmp >> 1;
            }
        }
        fd_v
    }

    pub fn as_bytes(&self) -> &[u8] {
        let size = core::mem::size_of::<Self>();
        unsafe { core::slice::from_raw_parts(self as *const _ as usize as *const u8, size) }
    }

    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        let size = core::mem::size_of::<Self>();
        unsafe { core::slice::from_raw_parts_mut(self as *mut _ as usize as *mut u8, size) }
    }
}
