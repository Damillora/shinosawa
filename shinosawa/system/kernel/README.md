# shinosawa::system::kernel

the beating heart of the shinosawa operating system.

> I sang against the beat of my heart
> 
> I danced against the beat of my heart
> 
> I laughed against the beat of my heart
> 
> I screamed against the beat of my heart
>
> —[mekurume](https://project-imas.wiki/Mekurume) by shinosawa hiro

# Features
- Uses the [Limine protocol](https://github.com/limine-bootloader/limine/blob/trunk/PROTOCOL.md) via the [limine](https://crates.io/crates/limine) crate.
- Supports x86_64 using the [x86_64](https://crates.io/crates/x86_64) crate.
- ACPI through the [acpi](https://crates.io/crates/acpi) crate.
- X2APIC interrupt controller support using the [x2apic](https://crates;io/crates/x2apic) crate.
- Basic linked list allocator.
- Basic thread scheduling.