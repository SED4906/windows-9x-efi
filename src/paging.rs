pub struct PageTable {
    pub entries: [PageTableEntry;1024]
}

impl PageTable {
    pub fn map_address(&mut self, physical_address: u32, virtual_address: u32, present: bool, read_write: bool, user: bool) {
        let top_10_bits = (virtual_address >> 22) as usize;
        if !self.entries[top_10_bits].present() {
            let new_page_table = uefi::boot::allocate_pages(uefi::boot::AllocateType::AnyPages, uefi::boot::MemoryType::LOADER_DATA, 1).unwrap().as_ptr() as u32;
            self.entries[top_10_bits] = PageTableEntry::new(new_page_table, true, true, true);
        }
        unsafe {(self.entries[top_10_bits].address() as *mut PageTable).as_mut()}.unwrap().map_address_1(physical_address, virtual_address, present, read_write, user);
    }

    fn map_address_1(&mut self, physical_address: u32, virtual_address: u32, present: bool, read_write: bool, user: bool) {
        let middle_10_bits = ((virtual_address >> 12) & 0x3FF) as usize;
        self.entries[middle_10_bits] = PageTableEntry::new(physical_address, present, read_write, user);
    }
}

pub struct PageTableEntry(u32);

impl PageTableEntry {
    pub fn present(&self) -> bool {
        self.0 & 1 == 1
    }

/*
    pub fn set_present(&mut self, value: bool) {
        if value {
            self.0 |= 1;
        } else {
            self.0 &= !1;
        }
    }

    pub fn read_write(&self) -> bool {
        self.0 & 2 == 2
    }

    pub fn set_read_write(&mut self, value: bool) {
        if value {
            self.0 |= 2;
        } else {
            self.0 &= !2;
        }
    }

    pub fn user(&self) -> bool {
        self.0 & 4 == 4
    }

    pub fn set_user(&mut self, value: bool) {
        if value {
            self.0 |= 4;
        } else {
            self.0 &= !4;
        }
    }
*/
    pub fn address(&mut self) -> u32 {
        self.0 & !0xFFF
    }
/*
    pub fn set_address(&mut self, value: u32) {
        self.0 &= 0xFFF;
        self.0 |= value & !0xFFF;
    }
*/
    pub fn new(address: u32, present: bool, read_write: bool, user: bool) -> Self {
        let mut data = address & !0xFFF;
        if present {data |= 1;}
        if read_write {data |= 2;}
        if user {data |= 4;}
        Self(data)
    }
}
