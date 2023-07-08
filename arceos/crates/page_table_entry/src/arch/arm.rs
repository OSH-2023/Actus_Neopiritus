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
        //const PT =       0b01 << 0;
        /// Privileged execute-never (PXN) attribute
        //const PXN =   1 << 2;
        /// Non-secure bit
        //const NS =   1 << 3;
        const XN_PVA=   1<<0;
        const PTY=      1<<1;
        const B=        1<<2;
        const C=        1<<3;
        const AP0=      1<<4;
        const AP1=      1<<5;
        const STEX=     0b111<<6;
        const AP2=      1<<9;
        const S=        1<<10;
        const nG=       1<<11;


        const BTEX=     0b111<<12;
        const XN=       1<<15;
       
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
    const ATTR_INDEX_MASK: u32 = 0b111;

    const fn from_mem_type(mem_type: MemType) -> Self {
        let mut bits:u32=Self::AP0.bits();
        if matches!(mem_type, MemType::Normal) {
            bits|=7<<2;
            bits|=1<<12;    
            
        }.
        else{
            bits|=1<<7;
            bits|=1<<13;
        }
        Self::from_bits_truncate(bits)
    }

    fn mem_type(&self) -> MemType {
        if self.contains(Self::PTY){
            if !self.contains(Self::C) && !self.contains(Self::B) {
                if (self.bits()>>6 & Self::ATTR_INDEX_MASK)==2{
                    return MemType::Device
                }
            }
            else if !self.contains(Self::C) && self.contains(Self::B){
                if (self.bits()>>6 & Self::ATTR_INDEX_MASK)==0{
                    return MemType::Device
                }

            }
        }
            else{
                if !self.contains(Self::C) && !self.contains(Self::B) {
                    if (self.bits()>>6 & Self::ATTR_INDEX_MASK)==2{
                        return MemType::Device
                    }
                }
                else if !self.contains(Self::C) && self.contains(Self::B){
                    if (self.bits()>>6 & Self::ATTR_INDEX_MASK)==0{
                        return MemType::Device
                    }
    
                }
        }
        return MemType::Normal
    }
}

impl From<DescriptorAttr> for MappingFlags {
    fn from(attr: DescriptorAttr) -> Self {
        let mut flags = Self::empty();
        if attr.contains(DescriptorAttr::XN_PVA)||attr.contains(DescriptorAttr::PTY){
            flags |= Self::READ;
        }
        if !attr.contains(DescriptorAttr::AP2) {
            flags |= Self::WRITE;
        }
        if attr.contains(DescriptorAttr::AP1) {
            flags |= Self::USER;
            if attr.contains(Self::PTY) {//smallpage
                if !attr.contains(DescriptorAttr::XN_PVA){
                    flags |= Self::EXECUTE;
                }
            }
            else{
                if !attr.contains(DescriptorAttr::XN){
                    flags |= Self::EXECUTE;
                }
            }
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
            attr |= Self::PTY;
        }
        if !flags.contains(MappingFlags::WRITE) {
            attr |= Self::AP2;
        }
        if flags.contains(MappingFlags::USER) {
            attr |= Self::AP1;
            
        } 
        if !flags.contains(MappingFlags::EXECUTE) {
                attr |= Self::XN_PVA | Self::PVA;
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
    const PHYS_ADDR_SMASK: usize = 0xffff_f000; // bits 12..31
    const PHYS_ADDR_BMASK: usize = 0xffff_0000; // bits 16..31
    /// Creates an empty descriptor with all bits set to zero.
    pub const fn empty() -> Self {
        Self(0)
    }
}

impl GenericPTE for A32PTE {
    fn new_page(paddr: PhysAddr, flags: MappingFlags, is_huge: bool) -> Self {//OK
        let mut attr: DescriptorAttr = DescriptorAttr::from(flags);
        if is_huge{
            attr|=DescriptorAttr::XN_PVA;
            attr&=~DescriptorAttr::STEX;
            Self((attr.bits() & ~Self::PHYS_ADDR_BMASK) | (paddr.as_usize() & Self::PHYS_ADDR_MASK) as u32)
        }
        else{
            Self((attr.bits() & ~Self::PHYS_ADDR_SMASK) | (paddr.as_usize() & Self::PHYS_ADDR_MASK) as u32)
        }
        
    }
    fn new_table(paddr: PhysAddr) -> Self {//OK
        let attr = DescriptorAttr::PTY;
        Self(attr.bits() | (paddr.as_usize() & Self::PHYS_ADDR_SMASK) as u32)
    }
    fn paddr(&self) -> PhysAddr {//OK
        if GenericPTE::is_huge(self){
            PhysAddr::from(self.0 as usize & Self::PHYS_ADDR_BMASK)
        }
        else{
            PhysAddr::from(self.0 as usize & Self::PHYS_ADDR_SMASK)
        }
    }
    fn flags(&self) -> MappingFlags {//OK
        DescriptorAttr::from_bits_truncate(self.0).into()
    }
    fn is_unused(&self) -> bool {//OK
        self.0 == 0
    }
    fn is_present(&self) -> bool {//OK
        let temp=DescriptorAttr::from_bits_truncate(self.0);
        temp.contains(DescriptorAttr::XN_PVA)||temp.contains(DescriptorAttr::PTY)
    }
    fn is_huge(&self) -> bool {//OK
        let temp=DescriptorAttr::from_bits_truncate(self.0);
        temp.contains(DescriptorAttr::XN_PVA) && !temp.contains(DescriptorAttr::PTY)
    }
    fn clear(&mut self) {//OK
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
