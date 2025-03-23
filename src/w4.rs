use alloc::vec;
use alloc::vec::Vec;
use uefi::println;

/// DoubleSpace Token
enum DSToken {
    Raw(u8),
    DepthCount(u16, u16),
    SectorBreak,
    End,
}

/// Read vector bit
/// # Parameters
/// input: A vector of compressed data.
/// index_bits: A mutable reference to the current index into the vector, measured in bits.
/// # Returns
/// A single bit, as a boolean.
fn read_vec_bit(input: &[u8], index_bits: &mut usize) -> bool {
    let result = input[*index_bits / 8] & (1 << (*index_bits % 8)) != 0;
    *index_bits += 1;
    result
}

/// DoubleSpace decoder - raw byte
/// # Parameters
/// input: A vector of compressed data.
/// index_bits: A mutable reference to the current index into the vector, measured in bits.
/// first: The most significant byte, read earlier.
/// # Returns
/// A single byte to append to the result.
fn ds_raw_byte(input: &[u8], index_bits: &mut usize, first: bool) -> u8 {
    let mut result = (first as u8) << 7;
    for bit in 0..7 {
        result |= (read_vec_bit(input, index_bits) as u8) << bit;
    }
    result
}

/// DoubleSpace decoder - read depth
/// # Parameters
/// input: A vector of compressed data.
/// index_bits: A mutable reference to the current index into the vector, measured in bits.
/// depth_base: The base value to add to the depth from this function.
/// depth_len: How many bits to read.
/// # Returns
/// A value telling what offset to begin copying from the sliding buffer.
fn ds_depth(input: &[u8], index_bits: &mut usize, depth_base: u16, depth_len: usize) -> u16 {
    let mut result = 0;
    for bit in 0..depth_len {
        result |= (read_vec_bit(input, index_bits) as u16) << bit;
    }
    result + depth_base
}

/// DoubleSpace decoder - read count (last step)
/// # Parameters
/// input: A vector of compressed data.
/// index_bits: A mutable reference to the current index into the vector, measured in bits.
/// count_base: The base value to add to the count from this function.
/// count_len: How many bits to read.
/// # Returns
/// A value counting how many bytes to copy from the sliding buffer.
fn ds_count1(input: &[u8], index_bits: &mut usize, count_base: u16, count_len: usize) -> u16 {
    let mut result = 0;
    for bit in 0..count_len {
        result |= (read_vec_bit(input, index_bits) as u16) << bit;
    }
    result + count_base
}

/// DoubleSpace decoder - read count
/// # Parameters
/// input: A vector of compressed data.
/// index_bits: A mutable reference to the current index into the vector, measured in bits.
/// # Returns
/// A value counting how many bytes to copy from the sliding buffer.
fn ds_count(input: &[u8], index_bits: &mut usize) -> u16 {
    let bases = [2, 3, 5, 9, 17, 33, 65, 129, 257];
    let first = read_vec_bit(&input, index_bits);
    if first {
        return bases[0];
    }
    let mut index = 1;
    while index < 9 {
        let next = read_vec_bit(&input, index_bits);
        if next {
            return ds_count1(input, index_bits, bases[index], index);
        }
        index += 1;
    }
    0 // illegal encoding
}

/// DoubleSpace decoder - read bits
/// # Parameters
/// input: A vector of compressed data.
/// index_bits: A mutable reference to the current index into the vector, measured in bits.
/// # Returns
/// A token that can be used to determine what to do.
fn ds_read_bits(input: &[u8], index_bits: &mut usize) -> DSToken {
    let first = read_vec_bit(&input, index_bits);
    let second = read_vec_bit(&input, index_bits);
    if first ^ second {
        return DSToken::Raw(ds_raw_byte(input, index_bits, first));
    } else if !(first && second) {
        let depth = ds_depth(input, index_bits, 0, 6);
        if depth == 0 {
            return DSToken::End;
        }
        let count = ds_count(input, index_bits);
        if count == 0 {
            panic!("invalid count 0 with depth {depth} at {index_bits} bits");
        }
        return DSToken::DepthCount(depth, count);
    } else {
        let third = read_vec_bit(&input, index_bits);
        let depth = if !third {
            ds_depth(input, index_bits, 64, 8)
        } else {
            ds_depth(input, index_bits, 320, 12)
        };
        if depth == 4415 {
            //print!("(sector break at {index_bits}) ");
            return DSToken::SectorBreak;
        }
        let count = ds_count(input, index_bits);
        if count == 0 {
            panic!("invalid count 0 with depth {depth} at {index_bits} bits");
        }
        return DSToken::DepthCount(depth, count);
    }
}

/// DoubleSpace decoder
/// # Parameters
/// input: A vector of compressed data.
/// # Returns
/// A vector of decompressed data.
fn ds_decode(input: &[u8]) -> Vec<u8> {
    let mut result = vec![];
    let mut index_bits = 0;
    loop {
        if result.len() >= 8192 {
            return result;
        }
        match ds_read_bits(&input, &mut index_bits) {
            DSToken::Raw(byte) => {
                result.push(byte);
            }
            DSToken::DepthCount(depth, count) => {
                for _ in 0..count {
                    let byte = result[result.len() - (depth as usize)];
                    result.push(byte);
                }
            }
            DSToken::SectorBreak => {
                //print!("(current length {}) ", result.len());
            }
            DSToken::End => {
                break;
            }
        }
    }
    result
}

fn w4_chunk_get_offset(input: &Vec<u8>, wx_vxd_offset: usize, index: usize) -> usize {
    u32::from_le_bytes(
        input[wx_vxd_offset + 16 + 4 * index..wx_vxd_offset + 16 + 4 * index + 4]
            .try_into()
            .unwrap(),
    ) as usize
}

/// W4 to W3
/// # Parameters
/// input: The compressed VxD archive.
/// # Returns
/// A decompressed W3 archive.
pub fn w4_to_w3(input: Vec<u8>) -> (Vec<u8>, usize) {
    let wx_vxd_offset = u32::from_le_bytes(input[0x3C..0x40].try_into().unwrap()) as usize;
    println!("Wx VxD header offset: {wx_vxd_offset:X}h");
    if input[wx_vxd_offset] != b'W'
        || !(input[wx_vxd_offset + 1] == b'3' || input[wx_vxd_offset + 1] == b'4')
    {
        panic!("invalid kernel image signature (expected W4 or W3)");
    }
    if input[wx_vxd_offset] == b'W' && input[wx_vxd_offset + 1] == b'3' {
        return (input[wx_vxd_offset..].into(), wx_vxd_offset);
    }
    let chunk_count = u16::from_le_bytes(
        input[wx_vxd_offset + 6..wx_vxd_offset + 8]
            .try_into()
            .unwrap(),
    ) as usize;
    let mut result = vec![];
    for index in 0..chunk_count {
        let offset = w4_chunk_get_offset(&input, wx_vxd_offset, index);
        if index < chunk_count - 1 && w4_chunk_get_offset(&input, wx_vxd_offset, index + 1) - offset == 8192 {
            result.append(&mut input[offset..offset + 8192].to_vec());
        } else {
            result.append(&mut ds_decode(&input[offset..]));
        }
    }
    (result, wx_vxd_offset)
}
