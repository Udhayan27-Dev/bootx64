use bootloader_api::info::MemoryRegionKind;
use bootloader_x86_64_common::legacy_memory_region::LegacyMemoryRegion;
use uefi::boot::{MemoryDescriptor, MemoryType};
use x86_64::PhysAddr;

#[derive(Debug,Copy,Clone)]
pub struct UefiMemoryDescriptor(pub MemoryDescriptor);

const PAGE_SIZE: u64 = 4096;


impl LegacyMemoryRegion for UefiMemoryDescriptor{
    fn start(&self) -> PhysAddr {
        PhysAddr::new(self.0.phys_start)
    }

    fn len(&self) -> u64 {
        self.0.page_count * PAGE_SIZE
    }

    fn kind(&self) -> MemoryRegionKind {
        match self.0.ty{
            MemoryType::CONVENTIONAL => MemoryRegionKind::Usable,
            other => MemoryRegionKind::UnknownUefi(other.0)
        }
    }

    fn usable_after_bootloader_exit(&self) -> bool {
        match self.0.ty {
            MemoryType::CONVENTIONAL => true,
            MemoryType::LOADER_CODE
            | MemoryType::LOADER_DATA 
            | MemoryType::BOOT_SERVICES_CODE
            | MemoryType::BOOT_SERVICES_DATA => {
                true
                //no need of these data locations anymore after the bootloader
                // control is passed to kernel 
            },
            MemoryType::RUNTIME_SERVICES_CODE | MemoryType::RUNTIME_SERVICES_DATA => {
                false
                //the UEFI standard specifies that these memory locations need to be preserved
                // //by bootloader and OS
            }

            _ => false
        }
    }
}