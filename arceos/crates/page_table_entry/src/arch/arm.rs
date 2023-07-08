//! arm32 VMSAv7-32 translation table format descriptors.

use core::fmt;
use memory_addr::PhysAddr;

use crate::{GenericPTE, MappingFlags};

bitflags::bitflags! {
    /// VMSAv7-32 Short-descriptor translation table second-level descriptor formats
    #[derive(Debug)]
    pub struct DescriptorAttr: u32 {
        // Attribute fields in stage 1 VMSAv7-32 Page descriptors:

        /// Whether the descriptor is pagetable
        const PT =       0b01 << 0;
        /// Privileged execute-never (PXN) attribute
        const PXN =   1 << 2;
        /// Non-secure bit
        const NS =   1 << 3;
        
       
    }
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum MemType {
    Device = 0,
    Normal = 1,
}

impl DescriptorAttr {
    #[allow(clippy::unusual_byte_groupings)]
    const ATTR_INDEX_MASK: u64 = 0b111_00;

    const fn from_mem_type(mem_type: MemType) -> Self {
        let mut bits = (mem_type as u64) << 2;
        if matches!(mem_type, MemType::Normal) {
            bits |= Self::INNER.bits() | Self::SHAREABLE.bits();
        }
        Self::from_bits_truncate(bits)
    }

    fn mem_type(&self) -> MemType {
        let idx = (self.bits() & Self::ATTR_INDEX_MASK) >> 2;
        match idx {
            0 => MemType::Device,
            1 => MemType::Normal,
            _ => panic!("Invalid memory attribute index"),
        }
    }
}

impl From<DescriptorAttr> for MappingFlags {
    fn from(attr: DescriptorAttr) -> Self {
        let mut flags = Self::empty();
        if attr.contains(DescriptorAttr::VALID) {
            flags |= Self::READ;
        }
        if !attr.contains(DescriptorAttr::AP_RO) {
            flags |= Self::WRITE;
        }
        if attr.contains(DescriptorAttr::AP_EL0) {
            flags |= Self::USER;
            if !attr.contains(DescriptorAttr::UXN) {
                flags |= Self::EXECUTE;
            }
        } else if !attr.intersects(DescriptorAttr::PXN) {
            flags |= Self::EXECUTE;
        }
        if attr.mem_type() == MemType::Device {
            flags |= Self::DEVICE;
        }
        flags
    }
}

impl From<MappingFlags> for DescriptorAttr {
    fn from(flags: MappingFlags) -> Self {
        let mut attr = if flags.contains(MappingFlags::DEVICE) {
            Self::from_mem_type(MemType::Device)
        } else {
            Self::from_mem_type(MemType::Normal)
        };
        if flags.contains(MappingFlags::READ) {
            attr |= Self::VALID;
        }
        if !flags.contains(MappingFlags::WRITE) {
            attr |= Self::AP_RO;
        }
        if flags.contains(MappingFlags::USER) {
            attr |= Self::AP_EL0 | Self::PXN;
            if !flags.contains(MappingFlags::EXECUTE) {
                attr |= Self::UXN;
            }
        } else {
            attr |= Self::UXN;
            if !flags.contains(MappingFlags::EXECUTE) {
                attr |= Self::PXN;
            }
        }
        attr
    }
}

/// A VMSAv8-64 translation table descriptor.
///
/// Note that the **AttrIndx\[2:0\]** (bit\[4:2\]) field is set to `0` for device
/// memory, and `1` for normal memory. The system must configure the MAIR_ELx
/// system register accordingly.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct A32PTE(u32);

impl A32PTE {
    const PHYS_ADDR_MASK: usize = 0xffff_f000; // bits 12..31

    /// Creates an empty descriptor with all bits set to zero.
    pub const fn empty() -> Self {
        Self(0)
    }
}

impl GenericPTE for A32PTE {
    fn new_page(paddr: PhysAddr, flags: MappingFlags, is_huge: bool) -> Self {
        let mut attr: DescriptorAttr = DescriptorAttr::from(flags) | DescriptorAttr::AF;
        if !is_huge {
            attr |= DescriptorAttr::NON_BLOCK;
        }
        Self(attr.bits() | (paddr.as_usize() & Self::PHYS_ADDR_MASK) as u64)
    }
    fn new_table(paddr: PhysAddr) -> Self {
        let attr = DescriptorAttr::NON_BLOCK | DescriptorAttr::VALID;
        Self(attr.bits() | (paddr.as_usize() & Self::PHYS_ADDR_MASK) as u64)
    }
    fn paddr(&self) -> PhysAddr {
        PhysAddr::from(self.0 as usize & Self::PHYS_ADDR_MASK)
    }
    fn flags(&self) -> MappingFlags {
        DescriptorAttr::from_bits_truncate(self.0).into()
    }
    fn is_unused(&self) -> bool {
        self.0 == 0
    }
    fn is_present(&self) -> bool {
        DescriptorAttr::from_bits_truncate(self.0).contains(DescriptorAttr::VALID)
    }
    fn is_huge(&self) -> bool {
        !DescriptorAttr::from_bits_truncate(self.0).contains(DescriptorAttr::NON_BLOCK)
    }
    fn clear(&mut self) {
        self.0 = 0
    }
}

impl fmt::Debug for A32PTE {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut f = f.debug_struct("A32PTE");
        f.field("raw", &self.0)
            .field("paddr", &self.paddr())
            .field("attr", &DescriptorAttr::from_bits_truncate(self.0))
            .field("flags", &self.flags())
            .finish()
    }
}
