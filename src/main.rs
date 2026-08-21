#![no_std] //neglects std RUST
#![no_main] //neglects std main in RUST
#![deny(unsafe_op_in_unsafe_fn)]
use core::{net::{IpAddr, Ipv4Addr}, ptr, slice, usize};

use bootloader_x86_64_common::{
    Kernel, RawFrameBufferInfo, SystemInfo, legacy_memory_region::LegacyFrameAllocator,
};
use bootloader_boot_config::BootConfig;
use uefi::{CStr8, CStr16, boot::{self, MemoryType, ScopedProtocol}, cstr16, proto::{ProtocolPointer, device_path::DevicePath, hii::config, loaded_image::LoadedImage, media::file::{File, FileAttribute, FileInfo}, network::pxe::{BaseCode, DhcpV4Packet}}};

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
    disk: cstr16!("kernel-x86_64"),
    tftp: cstr8!("kernel-x86_64"),
};

const CONFIG_FILE: BootFile = BootFile{
    disk: cstr16!("disk.json"),
    tftp: cstr8!("tftp.json"),
};

const RAMDISK_FILE: BootFile = BootFile{
    disk: cstr16!("ramdisk"),
    tftp: cstr8!("ramdisk"),
};

#[entry]
fn main() -> Status{
    let mut boot_mode = BootMode::Disk;
    let mut kernel = load_kernel(boot_mode);
    if kernel.is_none(){
        kernel = load_kernel(BootMode::Tftp);
    }
    let kernel = kernel.expect("failed to load kernel");
    //loads the config file from the specific disk and write to the RAM and return the starting address of the location
    let config_file = load_config_file(boot_mode);
    //temporary slot initialized tot store the Error if occur in the future becoz the screen in not loaded yet to display error storing the error help later t
    let mut error_loading_conifg: Option<serde_json_core::de::Error> = None;
    let mut config:BootConfig = match config_file
        //it is used to refernce the value inside the Option so the config_file remains original
        .as_deref()
        //this serde_json_core is the is key to convert the bytes into the json format and match it ot the BootCOnfig Struct
        .map(serde_json_core::from_slice)
        .transpose()
    {
            Ok(data) => data.unwrap_or_default().0,
            Err(err) => {
                error_loading_conifg = Some(err);
                //if the file does not exist the frame is set to default config
                Default::default()
            }
    };
}

fn load_config_file(boot_mode: BootMode) -> Option<&'static mut [u8]>{
    load_file_from_boot_method(&CONFIG_FILE, boot_mode)
}

fn load_ramdisk(boot_mode: BootMode) -> Option<&'static mut [u8]>{ig_
    load_file_from_boot_method(&RAMDISK_FILE, boot_mode)
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
    let file_size = usize::try_from(file_info.file_size()).unwrap();

    let file_slice = allocate_loader_data(file_size);
    //this writes the file into the memory
    file.read(file_slice).unwrap();
    //returns the memory address
    Some(file_slice)
}

//In hardware memory management, RAM is not handed out byte-by-byte.
//It is divided into fixed-size chunks called pages

//In hardware memory management, RAM is not handed out byte-by-byte.
//It is divided into fixed-size chunks called pages
//4KiB(4096 bytes) is standard page size in x86_64
fn allocate_loader_data(size: usize) -> &'static mut [u8] {
    //this returns the pointer to the allocated memory in RAM
    let mut ptr = boot::allocate_pages(boot::AllocateType::AnyPages, MemoryType::LOADER_DATA, ((size - 1)/ 4096) + 1,)
        .expect("Failed to allocate memory for the file");

    //the allocated memory will be filled with garbage value,
    //so replacing it with zero to write our kernel file
    unsafe{ptr::write_bytes(ptr.as_ptr(), 0, size)};
    unsafe {slice::from_raw_parts_mut(ptr.as_ptr(), size)}
}

fn load_file_from_tftp_boot_server(name: &CStr8) -> Option<&'static mut[u8]> {
    let mut base_code = open_pxe_base_code()?;

    //To find the tftp boot server
    let mode = base_code.mode();
    let dhcpv4: &DhcpV4Packet = mode.dhcp_ack().as_ref();
    let server_ip = Ipv4Addr::from_octets(dhcpv4.bootp_si_addr);

    //determine the file size
    let file_size = base_code.tftp_get_file_size(&server_ip.into(), name).ok()?;
    let kernel_size = usize::try_from(file_size).expect("The file size should fit into the usize");

    //Allocating some memory for the kernel file in RAM
    let slice = allocate_loader_data(kernel_size);

    //load kernel file
    base_code
        .tftp_read_file(&server_ip.into(), name, Some(slice))
        .expect("Failed tot read kernel file from the TFTP boot server");
    Some(slice)
}

//ScopedProtocol is used to close the connection when the variable goes out of scope
fn open_pxe_base_code() -> Option<boot::ScopedProtocol<BaseCode>> {
    let base_code = locate_and_open_protocol_from_image_device_path::<BaseCode>()?;
    //.mode returns the snapshot of the Network cards's current operations.
    // .dhcp_ack_received is to check whether any dhcp server is connected or not returns true if connection made ,then assigns an IP addr.
    base_code.mode().dhcp_ack_received().then_some(base_code)
}

// Protocol pointer :
fn locate_and_open_protocol_from_image_device_path<P: ProtocolPointer + ?Sized>()
-> Option<boot::ScopedProtocol<P>> {
    let image_handle = boot::image_handle();//this returns the unqine ID of the image(here our rust's uefi file)
    let loaded_image = boot::open_protocol_exclusive::<LoadedImage>(image_handle).ok()?;//this gives access to the hardware we were loaded
    let device_handle = loaded_image.device()?;//this returns the Unique ID of the device where the image is booted like USB or hardrive etc
    let device_path = boot::open_protocol_exclusive::<DevicePath>(device_handle).ok()?;//this returns the url like path of the booted device
   //In C, passing a pointer that gets modified requires passing a "pointer to a pointer" (DevicePath**). In Rust, the safe equivalent of a pointer to a pointer is a mutable reference to a reference (&mut &T).
   //By writing &mut &*device_path, we satisfy Rust's borrow checker while allowing the UEFI firmware to safely advance the pointer in memory as it searches the hardware tree.
    let handle = boot::locate_device_path::<P>(&mut &*device_path).ok()?;
    boot::open_protocol_exclusive::<P>(handle).ok()
}
