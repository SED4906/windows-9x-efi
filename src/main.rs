#![no_main]
#![no_std]

use core::panic::PanicInfo;

use alloc::vec::Vec;
use uefi::prelude::*;
use uefi::println;
use uefi::CString16;
use uefi::fs::{FileSystem, FileSystemResult};
use uefi::proto::media::fs::SimpleFileSystem;
use uefi::boot::{self, ScopedProtocol};

extern crate alloc;

mod w4;
mod w3;
mod le;
mod paging;

fn read_file(path: &str) -> FileSystemResult<Vec<u8>> {
    let path: CString16 = CString16::try_from(path).unwrap();
    let fs: ScopedProtocol<SimpleFileSystem> = boot::get_image_file_system(boot::image_handle()).unwrap();
    let mut fs = FileSystem::new(fs);
    fs.read(path.as_ref())
}

#[entry]
fn main() -> Status {
    uefi::helpers::init().unwrap();

    let registry = read_file("WINDOWS\\SYSTEM.DAT").unwrap();
    let kernel = read_file("WINDOWS\\SYSTEM\\VMM32.VXD").unwrap();

    println!("registry size: {} bytes", registry.len());
    println!("kernel size: {} bytes", kernel.len());

    let (kernel_decompressed, offset) = w4::w4_to_w3(kernel);
    println!("decompressed kernel size: {} bytes", kernel_decompressed.len());

    w3::w3_load_vxds(kernel_decompressed, offset);

    loop {}
    //Status::SUCCESS
}

#[panic_handler]
fn rust_panic(info: &PanicInfo) -> ! {
    uefi::println!("{info}");
    loop {}
}
