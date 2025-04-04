
/// Common instructions
pub mod instruct;
/// Interuppt handling
pub mod interrupt;
/// CPU-related functions
pub mod cpu;
/// Memory paging
pub mod paging;

/// x86_64 GDT setup
mod gdt;
/// Intel APIC
mod apic;
/// Frame Allocator
mod frame_alloc;