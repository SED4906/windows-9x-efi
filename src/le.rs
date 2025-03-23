use alloc::string::String;
use uefi::println;

use crate::paging::PageTable;

pub fn load_le(input: &[u8], header_size: usize, virtual_address_base: u32, page_table: &mut PageTable) -> u32 {
    if input[0] != b'L' || input[1] != b'E' {
        panic!("not an LE signature: {:X}h {:X}h (expected 4Ch 45h)", input[0], input[1]);
    }
    let object_table_offset = u32::from_le_bytes(input[0x40..0x44].try_into().unwrap()) as usize;
    let object_table_entries = u32::from_le_bytes(input[0x44..0x48].try_into().unwrap()) as usize;
    let page_map_offset = u32::from_le_bytes(input[0x48..0x4C].try_into().unwrap()) as usize;
    println!("{object_table_entries} objects");

    let mut object_cursor = header_size;

    let mut virtual_address_page = virtual_address_base;

    for index in 0..object_table_entries {
        let virtual_segment_size = u32::from_le_bytes(input[object_table_offset + index * 0x18..object_table_offset + index * 0x18 + 4].try_into().unwrap()) as usize;
        let base_address = u32::from_le_bytes(input[object_table_offset + index * 0x18 + 4..object_table_offset + index * 0x18 + 8].try_into().unwrap()) as usize;
        let object_flags = u32::from_le_bytes(input[object_table_offset + index * 0x18 + 8..object_table_offset + index * 0x18 + 12].try_into().unwrap());
        let page_map_index = u32::from_le_bytes(input[object_table_offset + index * 0x18 + 12..object_table_offset + index * 0x18 + 16].try_into().unwrap()) as usize;
        let page_map_entries = u32::from_le_bytes(input[object_table_offset + index * 0x18 + 16..object_table_offset + index * 0x18 + 20].try_into().unwrap()) as usize;
        let segment_name = String::from_utf8_lossy(&input[object_table_offset + index * 0x18 + 20..object_table_offset + index * 0x18 + 24]);
        let segment_name = segment_name.trim_matches(['\0','�']);
        println!("segment {segment_name}: size {virtual_segment_size:X}h, address {base_address:X}h, flags {object_flags:X}h, index {page_map_index} entries {page_map_entries}");
        let allocated_pages = uefi::boot::allocate_pages(uefi::boot::AllocateType::AnyPages, uefi::boot::MemoryType::LOADER_DATA, page_map_entries).unwrap();
        unsafe { core::ptr::copy(input.as_ptr().byte_add(object_cursor), allocated_pages.as_ptr(), virtual_segment_size); }
        object_cursor += virtual_segment_size;

        let mut map_from_address = allocated_pages.as_ptr() as u32;
        for index in page_map_index-1..page_map_index-1+page_map_entries {
            let page_number = u32::from_be_bytes(input[page_map_offset + index * 4..page_map_offset + index * 4 + 4].try_into().unwrap()) >> 8;
            let map_to_address = virtual_address_page + page_number * 4096;
            page_table.map_address(map_from_address, map_to_address, true, true, false);
            map_from_address += 4096;
        }
        virtual_address_page += page_map_entries as u32 * 4096;
    }

    virtual_address_page
}
