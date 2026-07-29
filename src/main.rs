#![no_std] //neglects std RUST
#![no_main] //neglects std main in RUST
#![deny(unsafe_op_in_unsafe_fn)] 
use bootloader_x86_64_common::{
    Kernel, RawFrameBufferInfo, SystemInfo, legacy_memory_region::LegacyFrameAllocator,
};
use uefi::{CStr8, CStr16, cstr16};

#[derive(Debug,Clone,Copy)]
pub enum BootMode{
    Disk,
    Tftp,
}

struct BootFile{
    disk: &'static CStr16,
    tftp: &'static CStr8,
}

const KERNEL_FILE: BootFile = BootFile{
    disk: cstr16!("kernel_x86-64"),
    tftp: cstr8!("kernel_x86-64"),
};

const BOOT_CONFIG: BootFile = BootFile{
    disk: cstr!("disk.json"),
    tftp: cstr!("tftp.json"),
};

#[entry]
fn main() -> Status{
    let mut boot_mode = BootMode::Disk;
    let mut kernel = load_kernel(boot_mode);
    if kernel.is_none(){
        kernel = load_kernel(BootMode::Tftp);
    }
} 


fn load_kernel(boot_mode: BootMode) -> Option<Kernel<'static>>{
    let kernel_slice = load_file_from_boot_method(&KERNEL_FILE, boot_mode)?;
    Some(Kernel::parse(kernel_slice))
}


