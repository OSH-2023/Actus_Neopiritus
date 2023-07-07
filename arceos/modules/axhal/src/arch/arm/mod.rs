mod context;
pub(crate) mod trap;

use core::arch::asm;

use memory_addr::{PhysAddr, VirtAddr};
pub use self::context::{FpState, TaskContext, TrapFrame};   //doto

/// Allows the current CPU to respond to interrupts.
#[inline]
pub fn enable_irqs() {
    unsafe {
        asm!(
            "mrs {0}, cpsr",
            "bic {0}, {0}, #0xc0",
            "msr cpsr_c, {0}",
            out(reg) _,
        );
    }
}

/// Makes the current CPU to ignore interrupts.
#[inline]
pub fn disable_irqs() {
    unsafe {
        asm!(
            "mrs {0}, cpsr",
            "orr {0}, {0}, #0xc0",
            "msr cpsr_c, {0}",
            out(reg) _,
        );
    }
}

/// Returns whether the current CPU is allowed to respond to interrupts.
#[inline]
pub fn irqs_enabled() -> bool {
    let result : usize;
    unsafe {
        asm!("mrs {0}, cpsr", out(reg) result);
    }
    result & 0x80 == 0
}

/// Relaxes the current CPU and waits for interrupts.
///
/// It must be called with interrupts enabled, otherwise it will never return.
#[inline]
pub fn wait_for_irqs() {
    unsafe {    asm!("wfi");    }
}

/// Halt the current CPU.
#[inline]
pub fn halt() {
    disable_irqs();
    unsafe {    asm!("wfi");    }
}

/// Reads the register that stores the current page table root.
///
/// Returns the physical address of the page table root.
#[inline]
pub fn read_page_table_root() -> PhysAddr {
    let root : usize;
    unsafe {    //read ttbr0 to variable root
        asm!("mrc p15, 0, {0}, c2, c0, 0", out(reg) root);
    }
    PhysAddr::from(root & 0xFFFFC000)
}

/// Writes the register to update the current page table root.
///
/// # Safety
///
/// This function is unsafe as it changes the virtual memory address space.
pub unsafe fn write_page_table_root(root_paddr: PhysAddr) {
    let old_root = read_page_table_root();
    trace!("set page table root: {:#x} => {:#x}", old_root, root_paddr);
    if old_root != root_paddr {
        unsafe {    //write ttbr0 from 'root_paddr.as_usize()'
            asm!("mcr p15, 0, {0}, c2, c0, 0", in(reg) root_paddr.as_usize());
        }        
        flush_tlb(None);
    }
}

/// Flushes the TLB.
///
/// If `vaddr` is [`None`], flushes the entire TLB. Otherwise, flushes the TLB
/// entry that maps the given virtual address.
#[inline]
pub fn flush_tlb(vaddr: Option<VirtAddr>) {
    unsafe {
        if let Some(vaddr) = vaddr {
            let mva: usize = vaddr.as_usize() & 0xFFFF_F000;
            //Invalidate unified TLB entries by MVA all ASID
            asm!("mcr p15, 0, {0}, c8, c7, 3", in(reg) mva);
        } else {
            //Invalidate entire unified TLB
            asm!("mcr p15, 0, {0}, c8, c7, 0", in(reg) _);
        }
    }    
}

// /// Flushes the entire instruction cache.
// #[inline]
// pub fn flush_icache_all() {
//     unsafe { asm!("ic iallu; dsb sy; isb") };
// }

/// Sets the base address of the exception vector (writes `VBAR_EL1`).
#[inline]
pub fn set_exception_vector_base(vbar_el1: usize) {
    unsafe {
        asm!("mcr p15, 0, {0}, c12, c0, 0", in(reg) vbar_el1);
    }
}
