use alloc::{string::String, vec};
use uefi::println;

use crate::paging::PageTable;

struct LEObject {
    virtual_size: u32,
    reloc_base: u32,
    flags: u32,
    page_table_index: u32,
    page_table_entries: u32,
    reserved: [u8; 4],
}

struct UnalignedU32 {
    data: [u8; 4],
}

pub struct LinearExecutable<'a> {
    page_size: usize,
    object_table: &'a [LEObject],
    object_page_table: *const UnalignedU32,
    fixup_page_table: *const UnalignedU32,
    fixup_record_table: *const u8,
}

impl LinearExecutable<'_> {
    pub fn new(input: &[u8]) -> Self {
        if input[0] != b'L' || input[1] != b'E' {
            panic!(
                "not an LE signature: {:X}h {:X}h (expected 4Ch 45h)",
                input[0], input[1]
            );
        }
        let page_size = u32::from_le_bytes(input[0x28..0x2C].try_into().unwrap()) as usize;
        let object_table_offset =
            u32::from_le_bytes(input[0x40..0x44].try_into().unwrap()) as usize;
        let object_table_entries =
            u32::from_le_bytes(input[0x44..0x48].try_into().unwrap()) as usize;
        let object_page_table_offset =
            u32::from_le_bytes(input[0x48..0x4C].try_into().unwrap()) as usize;
        let fixup_page_table_offset =
            u32::from_le_bytes(input[0x68..0x6C].try_into().unwrap()) as usize;
        let fixup_record_table_offset =
            u32::from_le_bytes(input[0x6C..0x70].try_into().unwrap()) as usize;

        let object_table = unsafe {
            core::slice::from_raw_parts(
                &raw const input[object_table_offset] as *const LEObject,
                object_table_entries,
            )
        };
        let object_page_table = &raw const input[object_page_table_offset] as *const UnalignedU32;
        let fixup_page_table = &raw const input[fixup_page_table_offset] as *const UnalignedU32;
        let fixup_record_table = &raw const input[fixup_record_table_offset];
        Self {
            page_size,
            object_table,
            object_page_table,
            fixup_page_table,
            fixup_record_table,
        }
    }

    pub fn load(&self, header_size: usize, virtual_base: u32, page_table: &mut PageTable) -> u32 {
        let mut virtual_page = virtual_base;
        println!(
            "({} page size | {} objects)",
            self.page_size,
            self.object_table.len()
        );
        for object in self.object_table {
            let object_name = String::from_utf8_lossy(&object.reserved);
            let object_name = object_name.trim_matches(['\0', '�']);
            println!(
                "object {object_name}: size {:X}h, base {:X}h, flags {:X}h, index {} entries {}",
                object.virtual_size,
                object.reloc_base,
                object.flags,
                object.page_table_index,
                object.page_table_entries
            );
            let page_table_slice = unsafe {
                core::slice::from_raw_parts(
                    self.object_page_table
                        .add(object.page_table_index as usize - 1),
                    object.page_table_entries as usize,
                )
            };
            let fixup_page_table_slice = unsafe {
                core::slice::from_raw_parts(
                    self.fixup_page_table
                        .add(object.page_table_index as usize - 1),
                    object.page_table_entries as usize + 1,
                )
            };
            for index in 0..page_table_slice.len() {
                let page = u32::from_be_bytes(page_table_slice[index].data); // why is this big endian? is it even?
                let page_number = page >> 8;
                let page_flags = page & 0xFF;
                if page_flags == 0 {
                    // copy page from offset
                } else if page_flags == 1 {
                    // whatever an "iterated data page" is
                } else if page_flags == 2 || page_flags == 3 {
                    // zero-filled page
                } else if page_flags == 4 {
                    // range of pages?
                } else {
                    println!("??? page {:X}h flags {:X}h ???", page_number, page_flags);
                    panic!("that can't be right...");
                }
                let fixup_start = u32::from_le_bytes(fixup_page_table_slice[index].data) as usize;
                let fixup_limit = u32::from_le_bytes(fixup_page_table_slice[index+1].data) as usize;
                let fixup_records = unsafe { core::slice::from_raw_parts(self.fixup_record_table.add(fixup_start), fixup_limit - fixup_start) };
                let mut fixup_cursor = 0;
                while fixup_cursor < fixup_records.len() {
                    let source_type = fixup_records[fixup_cursor];
                    let target_flags = fixup_records[fixup_cursor+1];
                    fixup_cursor += 2;
                    let mut source_offsets = vec![];
                    let source_offset_count = if source_type & 0x20 == 0 {
                        let source_offset = i16::from_le_bytes([fixup_records[fixup_cursor],fixup_records[fixup_cursor+1]]);
                        source_offsets.push(source_offset);
                        fixup_cursor += 2;
                        0
                    } else {
                        let count = fixup_records[fixup_cursor];
                        fixup_cursor += 1;
                        count
                    };
                    let target_object_number = if target_flags & 0x40 == 0 {
                        let value = fixup_records[fixup_cursor] as u16;
                        fixup_cursor += 1;
                        value
                    } else {
                        let value = u16::from_le_bytes([fixup_records[fixup_cursor],fixup_records[fixup_cursor+1]]);
                        fixup_cursor += 2;
                        value
                    };
                    let target_offset = if target_flags & 0x10 == 0 {
                        let value = u16::from_le_bytes([fixup_records[fixup_cursor],fixup_records[fixup_cursor+1]]) as u32;
                        fixup_cursor += 2;
                        value
                    } else {
                        let value = u32::from_le_bytes([fixup_records[fixup_cursor],fixup_records[fixup_cursor+1],fixup_records[fixup_cursor+2],fixup_records[fixup_cursor+3]]);
                        fixup_cursor += 4;
                        value
                    };
                    for _ in 0..source_offset_count {
                        let source_offset = i16::from_le_bytes([fixup_records[fixup_cursor],fixup_records[fixup_cursor+1]]);
                        source_offsets.push(source_offset);
                        fixup_cursor += 2;
                    }
                    let target_object_name = String::from_utf8_lossy(&self.object_table[target_object_number as usize - 1].reserved);
                    let target_object_name = target_object_name.trim_matches(['\0', '�']);
                    println!("fixup type {source_type:X}h flags {target_flags:X}h target object {target_object_number:X}h ({target_object_name}) @ offset {target_offset:X}h");
                    for source_offset in source_offsets {
                        println!("  source offset @ {source_offset:X}h");
                    }
                }
            }
            virtual_page += object.page_table_entries * 4096;
        }
        virtual_page
    }
}

fn set_unaligned_u32(address: u32, value: u32) {
    unsafe {
        *(address as *mut UnalignedU32) = UnalignedU32 {
            data: value.to_le_bytes(),
        }
    };
}
