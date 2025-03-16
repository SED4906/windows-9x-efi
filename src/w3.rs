use alloc::{string::String, vec::Vec};
use uefi::println;

/// Load W3 VxD drivers
/// # Parameters
/// input: The W3 archive.
/// # Returns
/// VxD
pub fn w3_load_vxds(input: Vec<u8>) {
    if input[0] != b'W' || !(input[1] == b'3') {
        panic!("invalid kernel image format (expected W3)");
    }
    let vxd_count = u16::from_le_bytes(input[4..6].try_into().unwrap());
    for index in 0..(vxd_count as usize) {
        let entry = &input[16 + 16*index..16 + 16*index + 16];
        let name = String::from_utf8(entry[0..8].trim_ascii().into()).unwrap();
        let file_offset = u32::from_le_bytes(entry[8..12].try_into().unwrap());
        let header_size = u32::from_le_bytes(entry[12..16].try_into().unwrap());
        println!("{name}: offset {file_offset:x}, header size {header_size}");
        // (actually load VxD)
    }
}
