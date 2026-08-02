#![no_std] //neglects std RUST
#![no_main] //neglects std main in RUST
#![deny(unsafe_op_in_unsafe_fn)] 
use bootloader_x86_64_common::{
    Kernel, RawFrameBufferInfo, SystemInfo, legacy_memory_region::LegacyFrameAllocator,
};
use uefi::{CStr8, CStr16, boot, cstr16, proto::media::file::{File, FileAttribute, FileInfo}};

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

fn load_file_from_boot_method(filename: &BootFile, boot_mode: BootMode) -> Option<&'static mut [u8]>{
    match boot_mode{
        BootMode::Disk => load_file_from_disk(filename.disk),
        BootMode::Tftp => load_file_from_tftp_boot_server(filename.tftp),
    }
}
=
fn load_file_from_disk(filename: &CStr16) -> Option<&'static mut [u8]>{
    let mut file_system = boot::get_image_file_system(boot::image_handle()).ok()?;
    
    let mut root = file_system.open_volume().unwrap();
    let file_handle_result = root.open(filename, FileMode::Read, FileAttribute::empty());
    let file_handle = file_handle_result.ok()?;

    let mut file = match file_handle.into_type().unwrap() {
        uefi::proto::media::file::FileType::Regular(f) => f,
        uefi::proto::media::file::FileType::Dir(_) => panic!(),
    };

    let mut buf = [0;500];
    let file_info: &mut FileInfo = file.get_info(&mut buf).unwrap();
    let file_size = usize::try_from(file_info.file_size()).unwrap();\

    let file_slice = allocate_loader_data(file_size);
    file.read(file_slice).unwrap();

    Some(file_slice)
        
}



