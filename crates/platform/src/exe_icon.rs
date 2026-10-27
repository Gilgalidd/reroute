//! The icon of a Windows program, read from its resources.
//!
//! Windows browsers seldom ship an icon file: the registry points at the
//! program itself (`"…\chrome.exe",0`), whose icon lives in its resource
//! section as an icon group (`RT_GROUP_ICON`) that lists the images
//! (`RT_ICON`) it is made of. This reads that section, and only that
//! section, and puts the first group back together as an `.ico` file, which
//! the web view shows like any other icon.
//!
//! The program is a file Reroute did not write: every offset and count is
//! checked before use, every read and the result are bounded, nothing is
//! run, and the whole of it is safe Rust.

use std::io::{Read, Seek, SeekFrom};

/// Largest resource section read. Browsers' are a few hundred KiB.
const MAX_SECTION: u32 = 32 * 1024 * 1024;

/// Largest `.ico` file put together, the size limit of icon files.
const MAX_ICO: usize = 2 * 1024 * 1024;

/// Most images taken from one icon group.
const MAX_IMAGES: usize = 64;

/// Most sections read from the section table; the format allows 96.
const MAX_SECTIONS: u16 = 96;

/// Resource type of one image of an icon.
const RT_ICON: u32 = 3;

/// Resource type of an icon group, the list of its images.
const RT_GROUP_ICON: u32 = 14;

/// The bit of a directory entry's offset that says it leads to another
/// directory rather than to data.
const SUBDIRECTORY: u32 = 0x8000_0000;

const NOT_A_PROGRAM: &str = "not a Windows program";
const TRUNCATED: &str = "the program file is truncated";
const NO_ICON: &str = "the program holds no icon";
const TOO_LARGE: &str = "the program's resources are too large";

/// The first icon of the program in `file`, as the bytes of an `.ico` file.
pub fn first_icon<F: Read + Seek>(file: &mut F) -> Result<Vec<u8>, &'static str> {
    let resources = Resources::read(file)?;
    let group = resources.find(RT_GROUP_ICON, None).ok_or(NO_ICON)?;
    // The group: a reserved word, its type (1 for icons), the number of
    // images, then 14 bytes for each.
    if u16_at(group, 2) != Some(1) {
        return Err(NO_ICON);
    }
    let count = usize::from(u16_at(group, 4).ok_or(NO_ICON)?).min(MAX_IMAGES);
    let mut images = Vec::new();
    let mut total = 0;
    for index in 0..count {
        let entry = 6 + 14 * index;
        let Some(head) = group.get(entry..entry + 8) else {
            break;
        };
        let Some(id) = u16_at(group, entry + 12) else {
            break;
        };
        // An image the group lists but the program lacks is left out.
        let Some(image) = resources.find(RT_ICON, Some(u32::from(id))) else {
            continue;
        };
        if total + image.len() > MAX_ICO {
            continue;
        }
        total += image.len();
        images.push((head, image));
    }
    ico(&images).ok_or(NO_ICON)
}

/// An `.ico` file: its header, a 16-byte entry for each image (the first
/// 8 bytes of the group's entry, then the image's size and offset), then
/// the images.
fn ico(images: &[(&[u8], &[u8])]) -> Option<Vec<u8>> {
    if images.is_empty() {
        return None;
    }
    let mut file = vec![0, 0, 1, 0];
    file.extend_from_slice(&u16::try_from(images.len()).ok()?.to_le_bytes());
    let mut offset = 6 + 16 * images.len();
    for (head, image) in images {
        file.extend_from_slice(head);
        file.extend_from_slice(&u32::try_from(image.len()).ok()?.to_le_bytes());
        file.extend_from_slice(&u32::try_from(offset).ok()?.to_le_bytes());
        offset += image.len();
    }
    for (_, image) in images {
        file.extend_from_slice(image);
    }
    Some(file)
}

/// The resource section of a program.
struct Resources {
    /// The section's bytes, as stored in the file.
    data: Vec<u8>,
    /// The address (RVA) of the section's first byte once loaded.
    base: u32,
    /// Where in `data` the resource directory starts.
    root: usize,
}

impl Resources {
    /// Find the resource section through the headers, and read it.
    fn read<F: Read + Seek>(file: &mut F) -> Result<Self, &'static str> {
        let dos = read_at(file, 0, 64)?;
        if dos.get(..2) != Some(b"MZ".as_slice()) {
            return Err(NOT_A_PROGRAM);
        }
        let pe = u64::from(u32_at(&dos, 0x3c).ok_or(NOT_A_PROGRAM)?);
        let header = read_at(file, pe, 24)?;
        if header.get(..4) != Some(b"PE\0\0".as_slice()) {
            return Err(NOT_A_PROGRAM);
        }
        let sections = u16_at(&header, 6).ok_or(NOT_A_PROGRAM)?;
        let optional_size = u16_at(&header, 20).ok_or(NOT_A_PROGRAM)?;
        if sections > MAX_SECTIONS {
            return Err(NOT_A_PROGRAM);
        }
        let optional = read_at(file, pe + 24, usize::from(optional_size))?;
        // Where the data directories start, after the fields whose size
        // depends on the format: 32-bit (PE32) or 64-bit (PE32+).
        let directories = match u16_at(&optional, 0) {
            Some(0x10b) => 96,
            Some(0x20b) => 112,
            _ => return Err(NOT_A_PROGRAM),
        };
        // The resource table is the third directory: an address and a size.
        let count = u32_at(&optional, directories - 4).ok_or(NO_ICON)?;
        let address = u32_at(&optional, directories + 16).ok_or(NO_ICON)?;
        if count < 3 || address == 0 {
            return Err(NO_ICON);
        }
        let table_at = pe + 24 + u64::from(optional_size);
        let table = read_at(file, table_at, usize::from(sections) * 40)?;
        let (base, size, offset) = table
            .chunks_exact(40)
            .find_map(|section| section_holding(section, address))
            .ok_or(NO_ICON)?;
        if size > MAX_SECTION {
            return Err(TOO_LARGE);
        }
        let size = usize::try_from(size).map_err(|_| TOO_LARGE)?;
        let data = read_at(file, u64::from(offset), size)?;
        let root = usize::try_from(address - base).map_err(|_| TOO_LARGE)?;
        Ok(Self { data, base, root })
    }

    /// The data of the first language of the resource of type `kind` named
    /// `id`, or of the first of that type when `id` is `None`. Windows sorts
    /// the entries, so the first one is the one `,0` means in the registry.
    fn find(&self, kind: u32, id: Option<u32>) -> Option<&[u8]> {
        let (_, names) = self.entries(0)?.find(|(name, _)| *name == kind)?;
        let (_, languages) = self
            .entries(subdirectory(names)?)?
            .find(|(name, _)| id.is_none_or(|id| *name == id))?;
        let (_, data) = self.entries(subdirectory(languages)?)?.next()?;
        if data & SUBDIRECTORY != 0 {
            return None;
        }
        let entry = self.root.checked_add(usize::try_from(data).ok()?)?;
        let address = u32_at(&self.data, entry)?;
        let size = u32_at(&self.data, entry + 4)?;
        let start = usize::try_from(address.checked_sub(self.base)?).ok()?;
        let end = start.checked_add(usize::try_from(size).ok()?)?;
        self.data.get(start..end)
    }

    /// The entries of the directory at `offset` from the resource root:
    /// the name or id of each, and where it leads.
    fn entries(&self, offset: u32) -> Option<impl Iterator<Item = (u32, u32)> + '_> {
        let at = self.root.checked_add(usize::try_from(offset).ok()?)?;
        let named = u16_at(&self.data, at + 12)?;
        let ids = u16_at(&self.data, at + 14)?;
        let count = usize::from(named) + usize::from(ids);
        let first = at + 16;
        Some((0..count).map_while(move |index| {
            let entry = first + 8 * index;
            Some((u32_at(&self.data, entry)?, u32_at(&self.data, entry + 4)?))
        }))
    }
}

/// The address, stored size and file offset of `section`, if it holds the
/// address `address`.
fn section_holding(section: &[u8], address: u32) -> Option<(u32, u32, u32)> {
    let loaded_size = u32_at(section, 8)?;
    let base = u32_at(section, 12)?;
    let stored_size = u32_at(section, 16)?;
    let offset = u32_at(section, 20)?;
    let end = base.checked_add(loaded_size.max(stored_size))?;
    (base <= address && address < end).then_some((base, stored_size, offset))
}

/// Where a directory entry leads, if it is another directory.
fn subdirectory(offset: u32) -> Option<u32> {
    (offset & SUBDIRECTORY != 0).then_some(offset & !SUBDIRECTORY)
}

/// Read `len` bytes at offset `at` of `file`.
fn read_at<F: Read + Seek>(file: &mut F, at: u64, len: usize) -> Result<Vec<u8>, &'static str> {
    file.seek(SeekFrom::Start(at)).map_err(|_| TRUNCATED)?;
    let mut bytes = vec![0; len];
    file.read_exact(&mut bytes).map_err(|_| TRUNCATED)?;
    Ok(bytes)
}

fn u16_at(bytes: &[u8], at: usize) -> Option<u16> {
    let end = at.checked_add(2)?;
    Some(u16::from_le_bytes(bytes.get(at..end)?.try_into().ok()?))
}

fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    let end = at.checked_add(4)?;
    Some(u32::from_le_bytes(bytes.get(at..end)?.try_into().ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// Address of the resource section in the programs built below.
    const BASE: usize = 0x1000;

    fn push16(bytes: &mut Vec<u8>, value: usize) {
        let value = u16::try_from(value).unwrap();
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push32(bytes: &mut Vec<u8>, value: usize) {
        let value = u32::try_from(value).unwrap();
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn put16(bytes: &mut [u8], at: usize, value: usize) {
        let value = u16::try_from(value).unwrap();
        bytes[at..at + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn put32(bytes: &mut [u8], at: usize, value: usize) {
        let value = u32::try_from(value).unwrap();
        bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }

    /// A directory: its header, then its entries, all with ids.
    fn directory(bytes: &mut Vec<u8>, entries: &[(usize, usize)]) {
        bytes.extend_from_slice(&[0; 12]);
        push16(bytes, 0);
        push16(bytes, entries.len());
        for &(id, offset) in entries {
            push32(bytes, id);
            push32(bytes, offset);
        }
    }

    /// A resource section with one icon group, id 101, made of `images`
    /// with ids 1, 2…, laid out as a compiler writes it.
    fn resources(images: &[&[u8]]) -> Vec<u8> {
        let n = images.len();
        let sub = |offset: usize| offset | 0x8000_0000;
        let icon_names = 32;
        let icon_languages = icon_names + 16 + 8 * n;
        let group_names = icon_languages + 24 * n;
        let group_languages = group_names + 24;
        let icon_data = group_languages + 24;
        let group_data = icon_data + 16 * n;
        let mut image_at = group_data + 16;
        let group_at = image_at + images.concat().len();

        let mut bytes = Vec::new();
        let types = [(3, sub(icon_names)), (14, sub(group_names))];
        directory(&mut bytes, &types);
        let ids: Vec<(usize, usize)> = (0..n)
            .map(|i| (i + 1, sub(icon_languages + 24 * i)))
            .collect();
        directory(&mut bytes, &ids);
        for i in 0..n {
            directory(&mut bytes, &[(1033, icon_data + 16 * i)]);
        }
        directory(&mut bytes, &[(101, sub(group_languages))]);
        directory(&mut bytes, &[(1033, group_data)]);
        for image in images {
            push32(&mut bytes, BASE + image_at);
            push32(&mut bytes, image.len());
            bytes.extend_from_slice(&[0; 8]);
            image_at += image.len();
        }
        push32(&mut bytes, BASE + group_at);
        push32(&mut bytes, 6 + 14 * n);
        bytes.extend_from_slice(&[0; 8]);
        for image in images {
            bytes.extend_from_slice(image);
        }
        push16(&mut bytes, 0);
        push16(&mut bytes, 1);
        push16(&mut bytes, n);
        for (i, image) in images.iter().enumerate() {
            let side = u8::try_from(16 * (i + 1)).unwrap();
            bytes.extend_from_slice(&[side, side, 0, 0]);
            push16(&mut bytes, 1);
            push16(&mut bytes, 32);
            push32(&mut bytes, image.len());
            push16(&mut bytes, i + 1);
        }
        bytes
    }

    /// A 64-bit program whose only section holds `resources`.
    fn program(resources: &[u8]) -> Vec<u8> {
        let mut bytes = b"MZ".to_vec();
        bytes.resize(0x3c, 0);
        push32(&mut bytes, 0x40);
        bytes.extend_from_slice(b"PE\0\0");
        push16(&mut bytes, 0x8664);
        push16(&mut bytes, 1);
        bytes.extend_from_slice(&[0; 12]);
        push16(&mut bytes, 240);
        push16(&mut bytes, 0x22);
        // The optional header of a 64-bit program: 16 data directories, the
        // third of which locates the resources.
        let optional = bytes.len();
        bytes.resize(optional + 240, 0);
        put16(&mut bytes, optional, 0x20b);
        put32(&mut bytes, optional + 108, 16);
        put32(&mut bytes, optional + 128, BASE);
        put32(&mut bytes, optional + 132, resources.len());
        bytes.extend_from_slice(b".rsrc\0\0\0");
        push32(&mut bytes, resources.len());
        push32(&mut bytes, BASE);
        push32(&mut bytes, resources.len());
        push32(&mut bytes, 0x200);
        bytes.extend_from_slice(&[0; 16]);
        bytes.resize(0x200, 0);
        bytes.extend_from_slice(resources);
        bytes
    }

    fn icon_of(bytes: Vec<u8>) -> Result<Vec<u8>, &'static str> {
        first_icon(&mut Cursor::new(bytes))
    }

    #[test]
    fn the_first_group_becomes_an_ico_file() {
        let small = b"\x89PNG small".as_slice();
        let large = b"\x89PNG a larger image".as_slice();
        let ico = icon_of(program(&resources(&[small, large]))).unwrap();
        assert_eq!(&ico[..6], &[0, 0, 1, 0, 2, 0]);
        // Each entry: size in pixels, colours, planes, depth; then the
        // length and the offset of its image.
        assert_eq!(&ico[6..14], &[16, 16, 0, 0, 1, 0, 32, 0]);
        assert_eq!(u32_at(&ico, 14), Some(10));
        assert_eq!(u32_at(&ico, 18), Some(6 + 32));
        assert_eq!(&ico[22..30], &[32, 32, 0, 0, 1, 0, 32, 0]);
        assert_eq!(u32_at(&ico, 30), Some(19));
        assert_eq!(u32_at(&ico, 34), Some(6 + 32 + 10));
        assert_eq!(&ico[38..], [small, large].concat().as_slice());
    }

    #[test]
    fn what_is_not_a_program_with_an_icon_is_refused() {
        assert_eq!(icon_of(b"\x89PNG".to_vec()), Err(TRUNCATED));
        assert_eq!(icon_of(vec![0; 512]), Err(NOT_A_PROGRAM));
        assert_eq!(icon_of(program(&[])), Err(NO_ICON), "no resources");
        // A group whose only image is missing.
        let mut lost = resources(&[b"image".as_slice()]);
        let id = lost.len() - 2;
        lost[id] = 9;
        assert_eq!(icon_of(program(&lost)), Err(NO_ICON));
    }

    /// Whatever the file holds, reading it never panics: every prefix of a
    /// valid program, and the program with any one byte changed.
    #[test]
    fn a_damaged_program_never_panics() {
        let valid = program(&resources(&[b"one".as_slice(), b"two".as_slice()]));
        assert!(icon_of(valid.clone()).is_ok());
        for len in 0..valid.len() {
            let _ = icon_of(valid[..len].to_vec());
        }
        for at in 0..valid.len() {
            for value in [0x00, 0x7f, 0x80, 0xff] {
                let mut damaged = valid.clone();
                damaged[at] = value;
                let _ = icon_of(damaged);
            }
        }
    }

    /// A real program: Microsoft Edge, part of Windows.
    #[cfg(windows)]
    #[test]
    fn edge_has_an_icon() {
        const EDGE: &str = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe";
        let Ok(mut file) = std::fs::File::open(EDGE) else {
            return;
        };
        let ico = first_icon(&mut file).unwrap();
        assert_eq!(&ico[..4], &[0, 0, 1, 0]);
        assert!(u16_at(&ico, 4).unwrap() > 0);
    }
}

