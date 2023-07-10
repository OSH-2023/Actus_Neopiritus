//! arm specific page table structures.

use crate::{PageTable64, PagingMetaData};
use page_table_entry::arm::A32PTE;

/// Metadata of AArch32 page tables.
#[derive(Copy, Clone)]
pub struct A32PagingMetaData;

impl const PagingMetaData for A32PagingMetaData {
    const LEVELS: usize = 2;
    const PA_MAX_BITS: usize = 20;
    const VA_MAX_BITS: usize = 20;

    fn vaddr_is_valid(vaddr: usize) -> bool {
        let top_bits = vaddr >> Self::VA_MAX_BITS;
        top_bits == 0 || top_bits == 0xffff
    }
}

/// arm VMSAv7-A translation table.
pub type A32PageTable<I> = PageTable64<A32PagingMetaData, A32PTE, I>;
