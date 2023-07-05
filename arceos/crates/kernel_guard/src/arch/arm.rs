use core::arch::asm;

#[inline]
pub fn local_irq_save_and_disable() -> usize {
    let flags: usize;
    unsafe {
        asm!(
            "mrs {0}, cpsr",
            "orr {1}, {0}, #0x80",
            "msr cpsr_c, {1}",
            out(reg) flags,
            out(reg) _,
        );
    }
    flags
}

#[inline]
pub fn local_irq_restore(flags: usize) {
    unsafe {
        asm!(
            "msr cpsr_c, {0}",
            in(reg) flags,
        );
    }
}