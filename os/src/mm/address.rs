/// # 地址数据类型
/// `os/src/mm/address.rs`
/// ## 实现功能
/// ```
/// pub struct PhysAddr(pub usize);     // 物理地址 56bit
/// pub struct PhysPageNum(pub usize);  // 物理页号 44bit
/// pub struct VirtAddr(pub usize);     // 虚拟地址 39bit
/// pub struct VirtPageNum(pub usize);  // 虚拟页号 27bit
/// ```
//

use super::PageTableEntry;
use crate::config::{PAGE_SIZE, PAGE_SIZE_BITS};
use core::fmt::{self, Debug, Formatter};

/// 物理地址宽度：56bit
const PA_WIDTH_SV39: usize = 56;
/// 虚拟地址宽度：39bit
const VA_WIDTH_SV39: usize = 39;
/// 物理页号宽度：44bit
const PPN_WIDTH_SV39: usize = PA_WIDTH_SV39 - PAGE_SIZE_BITS;
/// 虚拟页号宽度：27bit
const VPN_WIDTH_SV39: usize = VA_WIDTH_SV39 - PAGE_SIZE_BITS;

/// ### 物理地址 56bit
/// ```
/// PhysAddr::floor(&self) -> PhysPageNum
/// PhysAddr::ceil(&self) -> PhysPageNum
/// PhysAddr::page_offset(&self) -> usize
/// PhysAddr::aligned(&self) -> bool
/// PhysAddr::get_mut<T>(&self) -> &'static mut T
/// ```
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct PhysAddr(pub usize);

/// ### 虚拟地址 39bit
/// ```
/// VirtAddr::floor(&self) -> PhysPageNum
/// VirtAddr::ceil(&self) -> PhysPageNum
/// VirtAddr::page_offset(&self) -> usize
/// VirtAddr::aligned(&self) -> bool
/// ```
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct VirtAddr(pub usize);

/// ### 物理页号 44bit
/// ```
/// PhysPageNum::get_pte_array(&self) -> &'static mut [PageTableEntry]
/// PhysPageNum::get_bytes_array(&self) -> &'static mut [u8]
/// PhysPageNum::get_mut<T>(&self) -> &'static mut T
/// ```
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct PhysPageNum(pub usize);

/// ### 虚拟页号 27bit
/// ```
/// VirtPageNum::indexes(&self) -> [usize; 3]
/// ```
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct VirtPageNum(pub usize);

impl Debug for VirtAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("VA:{:#x}", self.0))
    }
}
impl Debug for VirtPageNum {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("VPN:{:#x}", self.0))
    }
}
impl Debug for PhysAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("PA:{:#x}", self.0))
    }
}
impl Debug for PhysPageNum {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("PPN:{:#x}", self.0))
    }
}

/// T: {PhysAddr, VirtAddr, PhysPageNum, VirtPageNum}
/// T -> usize: T.0
/// usize -> T: usize.into()
/// 当我们为类型 U 实现了 From<T> Trait 之后，可以使用 U::from(_: T) 来从一个 T 类型的实例来构造一个 U 类型的实例
/// 当我们为类型 U 实现了 Into<T> Trait 之后，对于一个 U 类型的实例 u ，可以使用 u.into() 来将其转化为一个类型为 T 的实例

impl From<usize> for PhysAddr {
    /// 取 `usize` 的低56位作为物理地址
    fn from(v: usize) -> Self {
        Self(v & ((1 << PA_WIDTH_SV39) - 1))
    }
}
impl From<usize> for PhysPageNum {
    /// 取 `usize` 的低44位作为物理页号
    fn from(v: usize) -> Self {
        Self(v & ((1 << PPN_WIDTH_SV39) - 1))
    }
}
impl From<usize> for VirtAddr {
    /// 取 `usize` 的低39位作为虚拟地址
    fn from(v: usize) -> Self {
        Self(v & ((1 << VA_WIDTH_SV39) - 1))
    }
}
impl From<usize> for VirtPageNum {
    /// 取 `usize` 的低27位作为虚拟页号
    fn from(v: usize) -> Self {
        Self(v & ((1 << VPN_WIDTH_SV39) - 1))
    }
}
impl From<PhysAddr> for usize {
    fn from(v: PhysAddr) -> Self {
        v.0
    }
}
impl From<PhysPageNum> for usize {
    fn from(v: PhysPageNum) -> Self {
        v.0
    }
}
impl From<VirtAddr> for usize {
    fn from(v: VirtAddr) -> Self {
        v.0
    }
}
impl From<VirtPageNum> for usize {
    fn from(v: VirtPageNum) -> Self {
        v.0
    }
}

impl VirtAddr {
    /// 从虚拟地址计算虚拟页号（下取整）
    pub fn floor(&self) -> VirtPageNum {
        VirtPageNum(self.0 / PAGE_SIZE)
    }
    /// 从虚拟地址计算虚拟页号（下取整）
    pub fn ceil(&self) -> VirtPageNum {
        VirtPageNum((self.0 - 1 + PAGE_SIZE) / PAGE_SIZE)
    }
    /// 从虚拟地址获取页内偏移（物理地址的低12位）
    pub fn page_offset(&self) -> usize {
        self.0 & (PAGE_SIZE - 1)
    }
    /// 判断虚拟地址是否与页面大小对齐
    pub fn aligned(&self) -> bool {
        self.page_offset() == 0
    }
}
impl From<VirtAddr> for VirtPageNum {
    fn from(v: VirtAddr) -> Self {
        assert_eq!(v.page_offset(), 0);
        v.floor()
    }
}
impl From<VirtPageNum> for VirtAddr {
    fn from(v: VirtPageNum) -> Self {
        Self(v.0 << PAGE_SIZE_BITS)
    }
}
impl PhysAddr {
    /// 从物理地址计算物理页号（下取整）
    pub fn floor(&self) -> PhysPageNum {
        PhysPageNum(self.0 / PAGE_SIZE)
    }
    /// 从物理地址计算物理页号（上取整）
    pub fn ceil(&self) -> PhysPageNum {
        PhysPageNum((self.0 - 1 + PAGE_SIZE) / PAGE_SIZE)
    }
    /// 从物理地址获取页内偏移（物理地址的低12位）
    pub fn page_offset(&self) -> usize {
        self.0 & (PAGE_SIZE - 1)
    }
    /// 判断物理地址是否与页面大小对齐
    pub fn aligned(&self) -> bool {
        self.page_offset() == 0
    }
    /// 获取一个大小为 T 的切片
    pub fn get_mut<T>(&self) -> &'static mut T {
        unsafe { (self.0 as *mut T).as_mut().unwrap() }
    }
}
impl From<PhysAddr> for PhysPageNum {
    fn from(v: PhysAddr) -> Self {
        // 对于物理地址与页面大小不对其的情况不能使用类型转换，panic
        assert_eq!(v.page_offset(), 0);
        v.floor()
    }
}

// 从物理页号转换到物理地址只需左移 PAGE_SIZE_BITS 大小
impl From<PhysPageNum> for PhysAddr {
    fn from(v: PhysPageNum) -> Self {
        Self(v.0 << PAGE_SIZE_BITS)
    }
}

impl VirtPageNum {
    /// 取出虚拟页号的三级页索引，并按照从高到低的顺序返回
    pub fn indexes(&self) -> [usize; 3] {
        let mut vpn = self.0;
        let mut idx = [0usize; 3];
        for i in (0..3).rev() {
            idx[i] = vpn & 511; // 取出低9位
            vpn >>= 9;
        }
        idx
    }
}

// 在实现方面，都是先把物理页号转为物理地址 PhysAddr ，然后再转成 usize 形式的物理地址。
// 接着，我们直接将它转为裸指针用来访问物理地址指向的物理内存。
// 在返回值类型上附加了静态生命周期泛型 'static ，这是为了绕过 Rust 编译器的借用检查，
// 实质上可以将返回的类型也看成一个裸指针，因为它也只是标识数据存放的位置以及类型。
// 但与裸指针不同的是，无需通过 unsafe 的解引用访问它指向的数据，而是可以像一个正常的可变引用一样直接访问
impl PhysPageNum {
    /// 根据自己的PPN取出当前节点的页表项数组
    pub fn get_pte_array(&self) -> &'static mut [PageTableEntry] {
        let pa: PhysAddr = (*self).into();
        unsafe { core::slice::from_raw_parts_mut(pa.0 as *mut PageTableEntry, 512) }
    }
    /// 返回一个字节数组的可变引用，可以以字节为粒度对物理页帧上的数据进行访问
    pub fn get_bytes_array(&self) -> &'static mut [u8] {
        let pa: PhysAddr = (*self).into();
        unsafe { core::slice::from_raw_parts_mut(pa.0 as *mut u8, 4096) }
    }
    /// 获取一个恰好放在一个物理页帧开头的类型为 T 的数据的可变引用
    pub fn get_mut<T>(&self) -> &'static mut T {
        let pa: PhysAddr = (*self).into();
        unsafe { (pa.0 as *mut T).as_mut().unwrap() }
    }
}

pub trait StepByOne {
    fn step(&mut self);
}
impl StepByOne for VirtPageNum {
    fn step(&mut self) {
        self.0 += 1;
    }
}

#[derive(Copy, Clone)]
pub struct SimpleRange<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    l: T,
    r: T,
}
impl<T> SimpleRange<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    pub fn new(start: T, end: T) -> Self {
        assert!(start <= end, "start {:?} > end {:?}!", start, end);
        Self { l: start, r: end }
    }
    pub fn get_start(&self) -> T {
        self.l
    }
    pub fn get_end(&self) -> T {
        self.r
    }
}
impl<T> IntoIterator for SimpleRange<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    type Item = T;
    type IntoIter = SimpleRangeIterator<T>;
    fn into_iter(self) -> Self::IntoIter {
        SimpleRangeIterator::new(self.l, self.r)
    }
}
pub struct SimpleRangeIterator<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    current: T,
    end: T,
}
impl<T> SimpleRangeIterator<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    pub fn new(l: T, r: T) -> Self {
        Self { current: l, end: r }
    }
}
impl<T> Iterator for SimpleRangeIterator<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.current == self.end {
            None
        } else {
            let t = self.current;
            self.current.step();
            Some(t)
        }
    }
}
pub type VPNRange = SimpleRange<VirtPageNum>;
