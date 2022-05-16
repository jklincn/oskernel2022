pub const NAME_LIMIT:usize = 128;

#[repr(C)]
pub struct Dirent {
    d_ino: usize,
    d_off: isize,
    d_reclen: u16,
    d_type: u8,
    d_name: [u8;NAME_LIMIT],
}

impl Dirent{
    pub fn new()->Self{
        Self{
            d_ino:0,
            d_off:0,
            d_reclen: 0,
            d_type:0,
            d_name:[0;NAME_LIMIT],
        }
    }

    pub fn init(&mut self,name:&str){
        self.fill_name(name);
    }

    fn fill_name(&mut self, name:&str) {
        let len = name.len().min(NAME_LIMIT);
        let name_bytes = name.as_bytes();
        for i in 0..len {
            self.d_name[i] = name_bytes[i];
        }
        self.d_name[len] = 0;
    }

    pub fn as_bytes(&self) -> &[u8] {
        let size = core::mem::size_of::<Self>();
        unsafe {
            core::slice::from_raw_parts(
                self as *const _ as usize as *const u8,
                size,
            )
        }
    }
}

