use alloc::{string::String, vec::Vec};
use uefi::println;

use crate::paging::PageTable;

/// Load W3 VxD drivers
/// # Parameters
/// input: The W3 archive.
/// # Returns
/// VxD
pub fn w3_load_vxds(input: Vec<u8>, header_offset: usize) {
    if input[0] != b'W' || !(input[1] == b'3') {
        panic!("invalid kernel image signature (expected W3)");
    }
    let staging_page_table = unsafe { uefi::boot::allocate_pages(uefi::boot::AllocateType::AnyPages, uefi::boot::MemoryType::LOADER_DATA, 1).unwrap().cast::<PageTable>().as_mut() };

    let vxd_count = u16::from_le_bytes(input[4..6].try_into().unwrap());
    for index in 0..(vxd_count as usize) {
        let entry = &input[16 + 16*index..16 + 16*index + 16];
        let name = String::from_utf8_lossy(&entry[0..8]);
        let name = name.trim();
        let offset = (u32::from_le_bytes(entry[8..12].try_into().unwrap()) - (header_offset as u32)) as usize;
        let header_size = u32::from_le_bytes(entry[12..16].try_into().unwrap()) as usize;
        println!("{name} -- offset {offset:X}h, header size {header_size}");
        // (actually load VxD)
        crate::le::load_le(&input[offset..], header_size, 0xc0000000, staging_page_table);
    }
}
