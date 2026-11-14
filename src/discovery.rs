//! Shared, passive SDK discovery primitives.

use std::ffi::OsString;
#[cfg(windows)]
use std::io::{Seek, SeekFrom};
use std::path::PathBuf;
use std::{fs::File, io::Read, path::Path};

use crate::Arch;

/// The first nonempty SDK root requested by the environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootRequest {
    /// Environment variable that supplied the path.
    pub variable: &'static str,
    /// Requested path, preserved without Unicode conversion or filesystem validation.
    pub path: PathBuf,
}

/// Return the first nonempty root; `None` means no variable supplied a path.
pub(crate) fn first_root(vars: &[&'static str]) -> Option<RootRequest> {
    first_root_with(vars, |var| std::env::var_os(var))
}

/// Injected environment lookup; `None` means no variable supplied a nonempty value.
/// The first request remains authoritative even if its path is unusable.
pub(crate) fn first_root_with(
    vars: &[&'static str],
    mut get: impl FnMut(&str) -> Option<OsString>,
) -> Option<RootRequest> {
    vars.iter().find_map(|&variable| {
        let path = get(variable)?;
        if path.is_empty() {
            return None;
        }
        Some(RootRequest {
            variable,
            path: PathBuf::from(path),
        })
    })
}

/// Read native ELF (Linux) or PE (Windows) metadata without loading or executing it.
///
/// `None` means unreadable, truncated, foreign-format, unrecognized, or incompatible
/// metadata (including big-endian ELF and unsupported machine/class combinations).
pub(crate) fn binary_arch(path: &Path) -> Option<Arch> {
    let mut file = File::open(path).ok()?;
    let mut header = [0; 20];
    file.read_exact(&mut header).ok()?;
    // ELF identity/layout and machine constants are defined by the Linux UAPI:
    // https://github.com/torvalds/linux/blob/master/include/uapi/linux/elf.h
    // https://github.com/torvalds/linux/blob/master/include/uapi/linux/elf-em.h
    #[cfg(target_os = "linux")]
    if header.get(..4)? == b"\x7fELF" {
        if header.get(5) != Some(&1) || header.get(6) != Some(&1) {
            return None;
        }
        let machine = u16::from_le_bytes([*header.get(18)?, *header.get(19)?]);
        return match (*header.get(4)?, machine) {
            (1, 3) => Some(Arch::X86),
            (2, 62) => Some(Arch::X64),
            (2, 183) => Some(Arch::Arm64),
            _ => None,
        };
    }
    #[cfg(windows)]
    {
        // e_lfanew at 0x3c points beyond the DOS header to the PE signature and Machine.
        // https://learn.microsoft.com/en-us/windows/win32/debug/pe-format
        if header.get(..2)? != b"MZ" {
            return None;
        }
        file.seek(SeekFrom::Start(0x3c)).ok()?;
        let mut offset = [0; 4];
        file.read_exact(&mut offset).ok()?;
        let offset = u64::from(u32::from_le_bytes(offset));
        if offset < 0x40 || offset.checked_add(6)? > file.metadata().ok()?.len() {
            return None;
        }
        file.seek(SeekFrom::Start(offset)).ok()?;
        let mut pe = [0; 6];
        file.read_exact(&mut pe).ok()?;
        if pe.get(..4)? != b"PE\0\0" {
            return None;
        }
        match u16::from_le_bytes([*pe.get(4)?, *pe.get(5)?]) {
            0x014c => Some(Arch::X86),
            0x8664 => Some(Arch::X64),
            0xaa64 => Some(Arch::Arm64),
            _ => None,
        }
    }
    #[cfg(not(windows))]
    None
}

/// Read architecture evidence from every object/import member of a regular archive.
///
/// This checks native format, fixed headers, and declared table/payload extents;
/// it does not claim that all relocations, symbols, or ABI requirements will link.
#[cfg(any(feature = "cuda", feature = "nvsdk", test))]
pub(crate) fn archive_arch(path: &Path, format: object::BinaryFormat) -> Option<Arch> {
    use object::BinaryFormat;
    use object::read::{ReadCache, ReadRef, archive::ArchiveFile};
    if !matches!(format, BinaryFormat::Coff | BinaryFormat::Elf) {
        return None;
    }
    let cache = ReadCache::new(File::open(path).ok()?);
    let len = (&cache).len().ok()?;
    if (&cache).read_bytes_at(0, 8).ok()? != object::archive::MAGIC {
        return None;
    }
    let archive = ArchiveFile::parse(&cache).ok()?;
    if archive.is_thin() {
        return None;
    }
    let mut arch = None;
    for member in archive.members() {
        let member = member.ok()?;
        let (offset, size) = member.file_range();
        let end = offset.checked_add(size)?;
        // ReadCache ranges retain the declared length, so check physical EOF first,
        // including the regular archive's alignment byte.
        if end.checked_add(end % 2)? > len {
            return None;
        }
        let candidate = archive_member_arch(cache.range(offset, size), format)?;
        if arch.is_some_and(|arch| arch != candidate) {
            return None;
        }
        arch = Some(candidate);
    }
    arch
}

#[cfg(any(feature = "cuda", feature = "nvsdk", test))]
fn archive_member_arch<'data, R: object::read::ReadRef<'data>>(
    data: R,
    format: object::BinaryFormat,
) -> Option<Arch> {
    use object::{BinaryFormat, FileKind, elf, pe};
    match (format, FileKind::parse(data).ok()?) {
        (BinaryFormat::Coff, FileKind::Coff) => coff_object_arch::<pe::ImageFileHeader, R>(data),
        (BinaryFormat::Coff, FileKind::CoffBig) => {
            coff_object_arch::<pe::AnonObjectHeaderBigobj, R>(data)
        }
        (BinaryFormat::Coff, FileKind::CoffImport) => coff_import_arch(data),
        (BinaryFormat::Elf, FileKind::Elf32) => {
            elf_object_arch::<elf::FileHeader32<object::Endianness>, R>(data)
        }
        (BinaryFormat::Elf, FileKind::Elf64) => {
            elf_object_arch::<elf::FileHeader64<object::Endianness>, R>(data)
        }
        _ => None,
    }
}

#[cfg(any(feature = "cuda", feature = "nvsdk", test))]
fn checked_table(offset: u64, count: u64, stride: u64, len: u64) -> Option<()> {
    (offset.checked_add(count.checked_mul(stride)?)? <= len).then_some(())
}

#[cfg(any(feature = "cuda", feature = "nvsdk", test))]
fn checked_file_range(offset: u64, size: u64, len: u64) -> Option<()> {
    // Empty ranges require no physical bytes, irrespective of their unused offset.
    if size == 0 {
        Some(())
    } else {
        checked_table(offset, 1, size, len)
    }
}

#[cfg(any(feature = "cuda", feature = "nvsdk", test))]
fn coff_machine_arch(machine: object::pe::Machine) -> Option<Arch> {
    use object::pe;
    match machine {
        pe::IMAGE_FILE_MACHINE_I386 => Some(Arch::X86),
        pe::IMAGE_FILE_MACHINE_AMD64 => Some(Arch::X64),
        pe::IMAGE_FILE_MACHINE_ARM64 => Some(Arch::Arm64),
        _ => None,
    }
}

#[cfg(any(feature = "cuda", feature = "nvsdk", test))]
fn coff_object_arch<'data, C: object::read::coff::CoffHeader, R: object::read::ReadRef<'data>>(
    data: R,
) -> Option<Arch> {
    let mut offset = 0;
    let header = C::parse(data, &mut offset).ok()?;
    let len = data.len().ok()?;
    // An object has no PE optional header.
    if offset != u64::try_from(std::mem::size_of::<C>()).ok()? {
        return None;
    }
    let section_size = u64::try_from(std::mem::size_of::<object::pe::ImageSectionHeader>()).ok()?;
    checked_table(
        offset,
        u64::from(header.number_of_sections()),
        section_size,
        len,
    )?;
    for index in 0..u64::from(header.number_of_sections()) {
        let section_offset = offset.checked_add(index.checked_mul(section_size)?)?;
        let section = data
            .read_at::<object::pe::ImageSectionHeader>(section_offset)
            .ok()?;
        if let Some((offset, size)) = section.coff_file_range() {
            checked_file_range(u64::from(offset), u64::from(size), len)?;
        }
    }
    let symbol_offset = u64::from(header.pointer_to_symbol_table());
    let symbol_count = u64::from(header.number_of_symbols());
    if symbol_offset == 0 {
        if symbol_count != 0 {
            return None;
        }
    } else {
        let symbol_size = u64::try_from(std::mem::size_of::<C::SymbolBytes>()).ok()?;
        checked_table(symbol_offset, symbol_count, symbol_size, len)?;
        let string_offset = symbol_offset.checked_add(symbol_count.checked_mul(symbol_size)?)?;
        let bytes = data.read_bytes_at(string_offset, 4).ok()?;
        let string_size = u64::from(u32::from_le_bytes(bytes.try_into().ok()?));
        if string_size < 4 {
            return None;
        }
        checked_table(string_offset, 1, string_size, len)?;
    }
    coff_machine_arch(header.machine())
}

#[cfg(any(feature = "cuda", feature = "nvsdk", test))]
fn coff_import_arch<'data, R: object::read::ReadRef<'data>>(data: R) -> Option<Arch> {
    use object::{LittleEndian as LE, pe};
    let mut offset = 0;
    let header = pe::ImportObjectHeader::parse(data, &mut offset).ok()?;
    let size = u64::from(header.size_of_data.get(LE));
    checked_table(offset, 1, size, data.len().ok()?)?;
    if size < 2
        || !matches!(
            header.import_type(),
            pe::IMPORT_OBJECT_CODE | pe::IMPORT_OBJECT_DATA | pe::IMPORT_OBJECT_CONST
        )
        || !matches!(
            header.name_type(),
            pe::IMPORT_OBJECT_ORDINAL
                | pe::IMPORT_OBJECT_NAME
                | pe::IMPORT_OBJECT_NAME_NO_PREFIX
                | pe::IMPORT_OBJECT_NAME_UNDECORATE
                | pe::IMPORT_OBJECT_NAME_EXPORTAS
        )
        || header.name_type.get(LE).0 >> 5 != 0
    {
        return None;
    }
    coff_machine_arch(header.machine.get(LE))
}

#[cfg(any(feature = "cuda", feature = "nvsdk", test))]
fn elf_object_arch<'data, E: object::read::elf::FileHeader, R: object::read::ReadRef<'data>>(
    data: R,
) -> Option<Arch> {
    use object::elf;
    use object::read::elf::{ProgramHeader, SectionHeader};
    let header = E::parse(data).ok()?;
    let endian = header.endian().ok()?;
    let header_size = u64::try_from(std::mem::size_of::<E>()).ok()?;
    if !header.is_little_endian()
        || header.e_type(endian) != elf::ET_REL
        || header.e_version(endian) != u32::from(elf::EV_CURRENT.0)
        || u64::from(header.e_ehsize(endian)) != header_size
    {
        return None;
    }
    let len = data.len().ok()?;
    let sections = u64::from(header.shnum(endian, data).ok()?);
    let section_offset = header.e_shoff(endian).into();
    let section_size = u64::try_from(std::mem::size_of::<E::SectionHeader>()).ok()?;
    if sections > 0
        && (section_offset < header_size || u64::from(header.e_shentsize(endian)) != section_size)
    {
        return None;
    }
    checked_table(section_offset, sections, section_size, len)?;
    let programs = u64::from(header.phnum(endian, data).ok()?);
    let program_offset = header.e_phoff(endian).into();
    let program_size = u64::try_from(std::mem::size_of::<E::ProgramHeader>()).ok()?;
    if programs > 0
        && (program_offset < header_size || u64::from(header.e_phentsize(endian)) != program_size)
    {
        return None;
    }
    checked_table(program_offset, programs, program_size, len)?;
    for index in 0..sections {
        let offset = section_offset.checked_add(index.checked_mul(section_size)?)?;
        let section = data.read_at::<E::SectionHeader>(offset).ok()?;
        // SHT_NULL fields can contain extended table metadata, not section data.
        if section.sh_type(endian) != elf::SHT_NULL
            && let Some((offset, size)) = section.file_range(endian)
        {
            checked_file_range(offset, size, len)?;
        }
    }
    for index in 0..programs {
        let offset = program_offset.checked_add(index.checked_mul(program_size)?)?;
        let program = data.read_at::<E::ProgramHeader>(offset).ok()?;
        if program.p_type(endian) != elf::PT_NULL {
            let (offset, size) = program.file_range(endian);
            checked_file_range(offset, size, len)?;
        }
    }
    match (header.is_class_64(), header.e_machine(endian)) {
        (false, elf::EM_386) => Some(Arch::X86),
        (true, elf::EM_X86_64) => Some(Arch::X64),
        (true, elf::EM_AARCH64) => Some(Arch::Arm64),
        _ => None,
    }
}

#[cfg(test)]
pub(crate) fn test_archive(format: object::BinaryFormat, arch: Arch) -> Vec<u8> {
    test_archive_members(&[test_object(format, arch)])
}

#[cfg(test)]
fn test_archive_members(members: &[Vec<u8>]) -> Vec<u8> {
    let mut bytes = b"!<arch>\n".to_vec();
    for member in members {
        let header = format!(
            "{:<16}{:<12}{:<6}{:<6}{:<8}{:<10}\x60\n",
            "unit.o/",
            0,
            0,
            0,
            "100644",
            member.len()
        );
        assert_eq!(header.len(), 60);
        bytes.extend_from_slice(header.as_bytes());
        bytes.extend_from_slice(member);
        if member.len() % 2 != 0 {
            bytes.push(b'\n');
        }
    }
    bytes
}

#[cfg(test)]
fn test_object(format: object::BinaryFormat, arch: Arch) -> Vec<u8> {
    use object::BinaryFormat;
    let machine: u16 = match (format, arch) {
        (BinaryFormat::Coff, Arch::X86) => 0x014c,
        (BinaryFormat::Coff, Arch::X64) => 0x8664,
        (BinaryFormat::Coff, Arch::Arm64) => 0xaa64,
        (BinaryFormat::Elf, Arch::X86) => 3,
        (BinaryFormat::Elf, Arch::X64) => 62,
        (BinaryFormat::Elf, Arch::Arm64) => 183,
        _ => panic!("unsupported archive fixture format"),
    };
    if format == BinaryFormat::Coff {
        let mut bytes = vec![0; 60];
        bytes[..2].copy_from_slice(&machine.to_le_bytes());
        bytes[2..4].copy_from_slice(&1u16.to_le_bytes());
        bytes[20..25].copy_from_slice(b".text");
        bytes[56..60].copy_from_slice(&0x60000020u32.to_le_bytes());
        return bytes;
    }
    let is_32 = arch == Arch::X86;
    let (header_size, section_size) = if is_32 { (52, 40) } else { (64, 64) };
    let mut bytes = vec![0; header_size + section_size];
    bytes[..4].copy_from_slice(b"\x7fELF");
    bytes[4] = if is_32 { 1 } else { 2 };
    bytes[5] = 1;
    bytes[6] = 1;
    bytes[16..18].copy_from_slice(&1u16.to_le_bytes());
    bytes[18..20].copy_from_slice(&machine.to_le_bytes());
    bytes[20..24].copy_from_slice(&1u32.to_le_bytes());
    if is_32 {
        bytes[32..36].copy_from_slice(&52u32.to_le_bytes());
        bytes[40..42].copy_from_slice(&52u16.to_le_bytes());
        bytes[46..48].copy_from_slice(&40u16.to_le_bytes());
        bytes[48..50].copy_from_slice(&1u16.to_le_bytes());
    } else {
        bytes[40..48].copy_from_slice(&64u64.to_le_bytes());
        bytes[52..54].copy_from_slice(&64u16.to_le_bytes());
        bytes[58..60].copy_from_slice(&64u16.to_le_bytes());
        bytes[60..62].copy_from_slice(&1u16.to_le_bytes());
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

    fn check_binary(label: &str, bytes: &[u8], expected: Option<Arch>) {
        let id = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "vcv_rs_discovery_{label}_{}_{}.bin",
            std::process::id(),
            id
        ));
        std::fs::write(&path, bytes).expect("write binary fixture");
        let actual = binary_arch(&path);
        std::fs::remove_file(&path).expect("remove own binary fixture");
        assert_eq!(actual, expected, "{label}");
    }

    fn elf(class: u8, machine: u16) -> [u8; 20] {
        let mut bytes = [0; 20];
        bytes[..4].copy_from_slice(b"\x7fELF");
        bytes[4] = class;
        bytes[5] = 1;
        bytes[6] = 1;
        bytes[18..20].copy_from_slice(&machine.to_le_bytes());
        bytes
    }

    fn pe(machine: u16) -> [u8; 70] {
        let mut bytes = [0; 70];
        bytes[..2].copy_from_slice(b"MZ");
        bytes[0x3c..0x40].copy_from_slice(&64u32.to_le_bytes());
        bytes[64..68].copy_from_slice(b"PE\0\0");
        bytes[68..70].copy_from_slice(&machine.to_le_bytes());
        bytes
    }

    fn check_archive(
        label: &str,
        bytes: &[u8],
        format: object::BinaryFormat,
        expected: Option<Arch>,
    ) {
        use std::io::Write;
        let id = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "vcv_rs_archive_{label}_{}_{}.a",
            std::process::id(),
            id
        ));
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .expect("create own archive fixture");
        file.write_all(bytes).expect("write archive fixture");
        drop(file);
        let actual = archive_arch(&path, format);
        let runtime = binary_arch(&path);
        std::fs::remove_file(&path).expect("remove own archive fixture");
        assert_eq!(actual, expected, "{label}");
        if bytes.starts_with(b"!<arch>\n") {
            assert_eq!(
                runtime, None,
                "archive cannot supply runtime metadata: {label}"
            );
        }
    }

    #[test]
    fn archive_proof_requires_physical_native_members() {
        use object::BinaryFormat::{Coff, Elf};
        for format in [Coff, Elf] {
            for arch in [Arch::X86, Arch::X64, Arch::Arm64] {
                let bytes = test_archive(format, arch);
                check_archive("native", &bytes, format, Some(arch));
                check_archive(
                    "foreign_format",
                    &bytes,
                    if format == Coff { Elf } else { Coff },
                    None,
                );
            }
            for bytes in [
                Vec::new(),
                b"tagged x86_64 library".to_vec(),
                b"!<arch>\n".to_vec(),
            ] {
                check_archive("not_evidence", &bytes, format, None);
            }
            let native = test_object(format, Arch::X64);
            check_archive("bare_object", &native, format, None);
            check_archive(
                "mixed_arch",
                &test_archive_members(&[native.clone(), test_object(format, Arch::Arm64)]),
                format,
                None,
            );
            check_archive(
                "unknown_member",
                &test_archive_members(&[native.clone(), b"invalid member".to_vec()]),
                format,
                None,
            );
            check_archive(
                "mixed_format",
                &test_archive_members(&[
                    native,
                    test_object(if format == Coff { Elf } else { Coff }, Arch::X64),
                ]),
                format,
                None,
            );
        }
    }

    #[test]
    fn archives_reject_truncation_overflow_thin_and_missing_members() {
        use object::BinaryFormat::Coff;
        let bytes = test_archive(Coff, Arch::X64);
        check_archive(
            "unsupported_requested_format",
            &bytes,
            object::BinaryFormat::MachO,
            None,
        );
        for size in [7, 67, bytes.len() - 1] {
            check_archive("truncated", &bytes[..size], Coff, None);
        }
        let mut oversized = bytes.clone();
        oversized[48..58].copy_from_slice(b"9999999999");
        check_archive("oversized", &oversized, Coff, None);
        let mut thin = bytes.clone();
        thin[..8].copy_from_slice(b"!<thin>\n");
        check_archive("thin", &thin, Coff, None);
        let mut malformed_header = bytes;
        malformed_header[66] = b'?';
        check_archive("malformed_header", &malformed_header, Coff, None);
        let missing = std::env::temp_dir().join(format!(
            "vcv_rs_missing_archive_{}_{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        assert_eq!(archive_arch(&missing, Coff), None);
    }

    #[test]
    fn elf_archive_class_endian_version_and_table_bounds_must_agree() {
        use object::BinaryFormat::Elf;
        for (arch, machine) in [
            (Arch::X86, 62u16),
            (Arch::X86, 183),
            (Arch::X64, 3),
            (Arch::Arm64, 3),
        ] {
            let mut member = test_object(Elf, arch);
            member[18..20].copy_from_slice(&machine.to_le_bytes());
            check_archive("wrong_class", &test_archive_members(&[member]), Elf, None);
        }
        for offset in [5, 6, 20, 52, 58, 60] {
            let mut member = test_object(Elf, Arch::X64);
            member[offset] = if offset == 5 { 2 } else { 0xff };
            check_archive(
                "invalid_elf_header",
                &test_archive_members(&[member]),
                Elf,
                None,
            );
        }
        let mut member = test_object(Elf, Arch::X64);
        member[16..18].copy_from_slice(&3u16.to_le_bytes());
        check_archive("shared_member", &test_archive_members(&[member]), Elf, None);
        let mut member = test_object(Elf, Arch::X64);
        member[40..48].copy_from_slice(&u64::MAX.to_le_bytes());
        check_archive(
            "section_overflow",
            &test_archive_members(&[member]),
            Elf,
            None,
        );
    }

    fn import_object(arch: Arch) -> Vec<u8> {
        let coff = test_object(object::BinaryFormat::Coff, arch);
        let strings = b"function\0library.dll\0";
        let mut bytes = vec![0; 20];
        bytes[2..4].copy_from_slice(&0xffffu16.to_le_bytes());
        bytes[6..8].copy_from_slice(&coff[..2]);
        bytes[12..16].copy_from_slice(&(strings.len() as u32).to_le_bytes());
        bytes[18..20].copy_from_slice(&4u16.to_le_bytes());
        bytes.extend_from_slice(strings);
        bytes
    }

    fn big_object(arch: Arch) -> Vec<u8> {
        let coff = test_object(object::BinaryFormat::Coff, arch);
        let mut bytes = vec![0; 96];
        bytes[2..4].copy_from_slice(&0xffffu16.to_le_bytes());
        bytes[4..6].copy_from_slice(&2u16.to_le_bytes());
        bytes[6..8].copy_from_slice(&coff[..2]);
        bytes[12..28].copy_from_slice(&object::pe::ANON_OBJECT_HEADER_BIGOBJ_CLASS_ID.0);
        bytes[44..48].copy_from_slice(&1u32.to_le_bytes());
        bytes[56..96].copy_from_slice(&coff[20..60]);
        bytes
    }

    #[test]
    fn coff_archives_validate_short_import_bigobj_and_declared_tables() {
        use object::BinaryFormat::Coff;
        for arch in [Arch::X86, Arch::X64, Arch::Arm64] {
            for member in [import_object(arch), big_object(arch)] {
                check_archive(
                    "coff_variants",
                    &test_archive_members(&[member]),
                    Coff,
                    Some(arch),
                );
            }
        }
        for machine in [0xa641u16, 0xa64e, 0] {
            let mut regular = test_object(Coff, Arch::X64);
            regular[..2].copy_from_slice(&machine.to_le_bytes());
            let mut import = import_object(Arch::X64);
            import[6..8].copy_from_slice(&machine.to_le_bytes());
            let mut big = big_object(Arch::X64);
            big[6..8].copy_from_slice(&machine.to_le_bytes());
            for member in [regular, import, big] {
                check_archive(
                    "unsupported_coff_machine",
                    &test_archive_members(&[member]),
                    Coff,
                    None,
                );
            }
        }
        for offset in [6, 12, 18] {
            let mut member = import_object(Arch::X64);
            member[offset] = 0xff;
            check_archive(
                "invalid_import",
                &test_archive_members(&[member]),
                Coff,
                None,
            );
        }
        let mut member = import_object(Arch::X64);
        member.pop();
        check_archive(
            "truncated_import",
            &test_archive_members(&[member]),
            Coff,
            None,
        );
        let mut member = big_object(Arch::X64);
        member[12] = 0;
        check_archive(
            "invalid_bigobj_id",
            &test_archive_members(&[member]),
            Coff,
            None,
        );
        let mut member = big_object(Arch::X64);
        member[44..48].copy_from_slice(&u32::MAX.to_le_bytes());
        check_archive(
            "bigobj_table_overflow",
            &test_archive_members(&[member]),
            Coff,
            None,
        );
        let mut member = test_object(Coff, Arch::X64);
        member[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
        member[12..16].copy_from_slice(&1u32.to_le_bytes());
        check_archive(
            "coff_symbols_overflow",
            &test_archive_members(&[member]),
            Coff,
            None,
        );
    }

    #[test]
    fn archives_skip_canonical_metadata_and_check_padding() {
        use object::BinaryFormat::{Coff, Elf};
        let native = test_object(Coff, Arch::X64);
        let mut bytes = test_archive_members(&[vec![0; 4], vec![0; 8], native]);
        for offset in [8, 72] {
            bytes[offset..offset + 16].fill(b' ');
            bytes[offset] = b'/';
        }
        check_archive("coff_linker_members", &bytes, Coff, Some(Arch::X64));
        bytes.truncate(140);
        check_archive("metadata_only", &bytes, Coff, None);
        let mut member = test_object(Elf, Arch::X64);
        member.push(0);
        let mut bytes = test_archive_members(&[member]);
        check_archive("odd_member", &bytes, Elf, Some(Arch::X64));
        bytes.pop();
        check_archive("missing_padding", &bytes, Elf, None);
    }

    #[test]
    fn archive_declared_symbol_and_extended_section_tables_are_bounded() {
        use object::BinaryFormat::{Coff, Elf};
        let mut member = test_object(Coff, Arch::X64);
        member[8..12].copy_from_slice(&60u32.to_le_bytes());
        member[12..16].copy_from_slice(&1u32.to_le_bytes());
        member.extend_from_slice(&[0; 18]);
        member.extend_from_slice(&4u32.to_le_bytes());
        check_archive(
            "coff_symbol_and_string_tables",
            &test_archive_members(&[member.clone()]),
            Coff,
            Some(Arch::X64),
        );
        member[78..82].copy_from_slice(&u32::MAX.to_le_bytes());
        check_archive(
            "string_table_overflow",
            &test_archive_members(&[member]),
            Coff,
            None,
        );
        let mut member = test_object(Elf, Arch::X64);
        member[60..62].fill(0);
        member[96..104].copy_from_slice(&1u64.to_le_bytes());
        check_archive(
            "extended_section_count",
            &test_archive_members(&[member.clone()]),
            Elf,
            Some(Arch::X64),
        );
        member[96..104].copy_from_slice(&u64::MAX.to_le_bytes());
        check_archive(
            "extended_section_overflow",
            &test_archive_members(&[member]),
            Elf,
            None,
        );
    }

    fn elf_section_extent_object(arch: Arch, kind: u32, offset: u64, size: u64) -> Vec<u8> {
        let mut member = test_object(object::BinaryFormat::Elf, arch);
        let section = member.len();
        if arch == Arch::X86 {
            member.resize(section + 40, 0);
            member[48..50].copy_from_slice(&2u16.to_le_bytes());
            member[section + 4..section + 8].copy_from_slice(&kind.to_le_bytes());
            member[section + 16..section + 20]
                .copy_from_slice(&u32::try_from(offset).expect("ELF32 offset").to_le_bytes());
            member[section + 20..section + 24]
                .copy_from_slice(&u32::try_from(size).expect("ELF32 size").to_le_bytes());
        } else {
            member.resize(section + 64, 0);
            member[60..62].copy_from_slice(&2u16.to_le_bytes());
            member[section + 4..section + 8].copy_from_slice(&kind.to_le_bytes());
            member[section + 24..section + 32].copy_from_slice(&offset.to_le_bytes());
            member[section + 32..section + 40].copy_from_slice(&size.to_le_bytes());
        }
        member
    }

    fn elf_program_extent_object(
        arch: Arch,
        kind: u32,
        offset: u64,
        size: u64,
        memory: u64,
    ) -> Vec<u8> {
        let mut member = test_object(object::BinaryFormat::Elf, arch);
        let program = member.len();
        if arch == Arch::X86 {
            member.resize(program + 32, 0);
            member[28..32].copy_from_slice(
                &u32::try_from(program)
                    .expect("ELF32 table offset")
                    .to_le_bytes(),
            );
            member[42..44].copy_from_slice(&32u16.to_le_bytes());
            member[44..46].copy_from_slice(&1u16.to_le_bytes());
            member[program..program + 4].copy_from_slice(&kind.to_le_bytes());
            member[program + 4..program + 8]
                .copy_from_slice(&u32::try_from(offset).expect("ELF32 offset").to_le_bytes());
            member[program + 16..program + 20]
                .copy_from_slice(&u32::try_from(size).expect("ELF32 file size").to_le_bytes());
            member[program + 20..program + 24].copy_from_slice(
                &u32::try_from(memory)
                    .expect("ELF32 memory size")
                    .to_le_bytes(),
            );
        } else {
            member.resize(program + 56, 0);
            member[32..40].copy_from_slice(
                &u64::try_from(program)
                    .expect("ELF64 table offset")
                    .to_le_bytes(),
            );
            member[54..56].copy_from_slice(&56u16.to_le_bytes());
            member[56..58].copy_from_slice(&1u16.to_le_bytes());
            member[program..program + 4].copy_from_slice(&kind.to_le_bytes());
            member[program + 8..program + 16].copy_from_slice(&offset.to_le_bytes());
            member[program + 32..program + 40].copy_from_slice(&size.to_le_bytes());
            member[program + 40..program + 48].copy_from_slice(&memory.to_le_bytes());
        }
        member
    }

    #[test]
    fn coff_section_payload_extents_stay_inside_each_member() {
        use object::{BinaryFormat::Coff, pe};
        for (original, section) in [
            (test_object(Coff, Arch::X64), 20),
            (big_object(Arch::X64), 56),
        ] {
            let end = u32::try_from(original.len()).expect("small fixture");
            for (offset, size) in [(u32::MAX, 1), (end, 1), (end - 1, 2), (u32::MAX, u32::MAX)] {
                let mut member = original.clone();
                member[section + 16..section + 20].copy_from_slice(&size.to_le_bytes());
                member[section + 20..section + 24].copy_from_slice(&offset.to_le_bytes());
                check_archive(
                    "coff_raw_data_outside_member",
                    &test_archive_members(&[member]),
                    Coff,
                    None,
                );
            }
            let mut member = original.clone();
            member[section + 20..section + 24].copy_from_slice(&u32::MAX.to_le_bytes());
            check_archive(
                "coff_zero_raw_data",
                &test_archive_members(&[member]),
                Coff,
                Some(Arch::X64),
            );
            let mut member = original.clone();
            member[section + 16..section + 20].copy_from_slice(&u32::MAX.to_le_bytes());
            member[section + 20..section + 24].copy_from_slice(&u32::MAX.to_le_bytes());
            member[section + 36..section + 40]
                .copy_from_slice(&pe::IMAGE_SCN_CNT_UNINITIALIZED_DATA.0.to_le_bytes());
            check_archive(
                "coff_large_uninitialized",
                &test_archive_members(&[member]),
                Coff,
                Some(Arch::X64),
            );
            let mut member = original;
            member[section + 16..section + 20].copy_from_slice(&1u32.to_le_bytes());
            member[section + 20..section + 24].copy_from_slice(&end.to_le_bytes());
            member.push(0);
            check_archive(
                "coff_data_ends_at_member_eof",
                &test_archive_members(&[member.clone()]),
                Coff,
                Some(Arch::X64),
            );
            member.pop();
            check_archive(
                "coff_payload_truncated",
                &test_archive_members(&[member]),
                Coff,
                None,
            );
        }
    }

    #[test]
    fn elf_section_payload_extents_preserve_nobits_null_and_zero_data() {
        use object::{BinaryFormat::Elf, elf};
        for arch in [Arch::X86, Arch::X64] {
            let max = if arch == Arch::X86 {
                u64::from(u32::MAX)
            } else {
                u64::MAX
            };
            let end = if arch == Arch::X86 { 132 } else { 192 };
            for (offset, size) in [(max, 1), (end, 1), (end - 1, 2), (max, max)] {
                let member = elf_section_extent_object(arch, elf::SHT_PROGBITS.0, offset, size);
                check_archive(
                    "elf_section_outside_member",
                    &test_archive_members(&[member]),
                    Elf,
                    None,
                );
            }
            for (kind, size) in [
                (elf::SHT_NOBITS, max),
                (elf::SHT_NULL, max),
                (elf::SHT_PROGBITS, 0),
            ] {
                let member = elf_section_extent_object(arch, kind.0, max, size);
                check_archive(
                    "elf_non_file_backed_section",
                    &test_archive_members(&[member]),
                    Elf,
                    Some(arch),
                );
            }
            let mut member = elf_section_extent_object(arch, elf::SHT_PROGBITS.0, end, 1);
            member.push(0);
            check_archive(
                "elf_section_ends_at_member_eof",
                &test_archive_members(&[member.clone()]),
                Elf,
                Some(arch),
            );
            member.pop();
            check_archive(
                "elf_section_payload_truncated",
                &test_archive_members(&[member]),
                Elf,
                None,
            );
        }
    }

    #[test]
    fn elf_program_payload_extents_preserve_null_and_zero_file_size() {
        use object::{BinaryFormat::Elf, elf};
        for arch in [Arch::X86, Arch::X64] {
            let max = if arch == Arch::X86 {
                u64::from(u32::MAX)
            } else {
                u64::MAX
            };
            let end = if arch == Arch::X86 { 124 } else { 184 };
            for (offset, size) in [(max, 1), (end, 1), (end - 1, 2), (max, max)] {
                let member = elf_program_extent_object(arch, elf::PT_LOAD.0, offset, size, max);
                check_archive(
                    "elf_program_outside_member",
                    &test_archive_members(&[member]),
                    Elf,
                    None,
                );
            }
            for (kind, size) in [(elf::PT_NULL, max), (elf::PT_LOAD, 0)] {
                let member = elf_program_extent_object(arch, kind.0, max, size, max);
                check_archive(
                    "elf_non_file_backed_program",
                    &test_archive_members(&[member]),
                    Elf,
                    Some(arch),
                );
            }
            let mut member = elf_program_extent_object(arch, elf::PT_LOAD.0, end, 1, max);
            member.push(0);
            check_archive(
                "elf_program_ends_at_member_eof",
                &test_archive_members(&[member.clone()]),
                Elf,
                Some(arch),
            );
            member.pop();
            check_archive(
                "elf_program_payload_truncated",
                &test_archive_members(&[member]),
                Elf,
                None,
            );
        }
    }

    #[test]
    fn elf_machine_class_and_encoding_must_agree() {
        let native = cfg!(target_os = "linux");
        check_binary("elf_x86", &elf(1, 3), native.then_some(Arch::X86));
        check_binary("elf_x64", &elf(2, 62), native.then_some(Arch::X64));
        check_binary("elf_arm64", &elf(2, 183), native.then_some(Arch::Arm64));
        for (class, machine) in [(2, 3), (1, 62), (1, 183), (0, 62), (2, 0)] {
            check_binary("elf_wrong_class_or_machine", &elf(class, machine), None);
        }
        let mut bytes = elf(2, 62);
        bytes[5] = 2;
        check_binary("elf_big_endian", &bytes, None);
        bytes[5] = 0;
        check_binary("elf_unknown_encoding", &bytes, None);
        bytes[5] = 1;
        bytes[6] = 0;
        check_binary("elf_bad_version", &bytes, None);
        check_binary("elf_truncated", &elf(2, 62)[..19], None);
        check_binary("unrecognized", &[0; 20], None);
        check_binary("empty", &[], None);
    }

    #[test]
    fn pe_machine_and_checked_header_bounds_are_required() {
        let native = cfg!(windows);
        check_binary("pe_x86", &pe(0x014c), native.then_some(Arch::X86));
        check_binary("pe_x64", &pe(0x8664), native.then_some(Arch::X64));
        check_binary("pe_arm64", &pe(0xaa64), native.then_some(Arch::Arm64));
        check_binary("pe_unknown", &pe(0), None);
        let mut bytes = pe(0x8664);
        bytes[0x3c..0x40].copy_from_slice(&u32::MAX.to_le_bytes());
        check_binary("pe_bad_offset", &bytes, None);
        bytes = pe(0x8664);
        bytes[2..6].copy_from_slice(b"PE\0\0");
        bytes[6..8].copy_from_slice(&0x8664u16.to_le_bytes());
        bytes[0x3c..0x40].copy_from_slice(&2u32.to_le_bytes());
        check_binary("pe_overlapping_dos_header", &bytes, None);
        check_binary("pe_truncated_offset", &pe(0x8664)[..63], None);
        check_binary("pe_truncated_machine", &pe(0x8664)[..69], None);
        bytes = pe(0x8664);
        bytes[64] = b'X';
        check_binary("pe_bad_signature", &bytes, None);
    }

    #[test]
    fn first_nonempty_root_is_authoritative_even_when_invalid() {
        let vars = ["FIRST", "SECOND", "THIRD"];
        let mut visited = Vec::new();
        let request = first_root_with(&vars, |var| {
            visited.push(var.to_string());
            match var {
                "FIRST" => Some(OsString::new()),
                "SECOND" => Some(OsString::from("missing-sdk-root")),
                _ => Some(OsString::from("valid-sdk-root")),
            }
        })
        .expect("second variable is nonempty");
        assert_eq!(request.variable, "SECOND");
        assert_eq!(request.path, PathBuf::from("missing-sdk-root"));
        assert_eq!(visited, ["FIRST", "SECOND"]);
        assert!(first_root_with(&vars, |_| None).is_none());
        assert!(first_root_with(&vars, |_| Some(OsString::new())).is_none());
    }

    #[test]
    fn root_values_are_preserved_without_trimming_or_unicode_conversion() {
        let request = first_root_with(&["ROOT"], |_| Some(OsString::from(" ")))
            .expect("whitespace is a nonempty native path");
        assert_eq!(request.path, PathBuf::from(" "));
        #[cfg(windows)]
        let native = {
            use std::os::windows::ffi::OsStringExt;
            OsString::from_wide(&[0xd800])
        };
        #[cfg(unix)]
        let native = {
            use std::os::unix::ffi::OsStringExt;
            OsString::from_vec(vec![0xff])
        };
        #[cfg(any(windows, unix))]
        {
            let request =
                first_root_with(&["ROOT"], |_| Some(native.clone())).expect("non-Unicode root");
            assert_eq!(request.path.into_os_string(), native);
        }
    }

    #[test]
    #[ignore = "requires VCV_TEST_COFF_ARCHIVE and VCV_TEST_ELF_ARCHIVE actual x64 SDK archive paths"]
    fn actual_sdk_archives_require_native_x64_members() {
        let coff_path = PathBuf::from(
            std::env::var_os("VCV_TEST_COFF_ARCHIVE")
                .expect("set VCV_TEST_COFF_ARCHIVE to an actual x64 COFF SDK archive file"),
        );
        let elf_path = PathBuf::from(
            std::env::var_os("VCV_TEST_ELF_ARCHIVE")
                .expect("set VCV_TEST_ELF_ARCHIVE to an actual x64 ELF SDK archive file"),
        );
        for (variable, path, format) in [
            ("VCV_TEST_COFF_ARCHIVE", coff_path, object::BinaryFormat::Coff),
            ("VCV_TEST_ELF_ARCHIVE", elf_path, object::BinaryFormat::Elf),
        ] {
            assert!(
                path.is_file(),
                "{variable} must name an actual SDK archive file: {path:?}"
            );
            assert_eq!(
                archive_arch(&path, format),
                Some(Arch::X64),
                "{variable} must contain native x64 members in the expected format: {path:?}"
            );
        }
    }
}
