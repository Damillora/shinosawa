SECTIONS {
  . = 0x5000000;
  /* Define an output section ".text". */
  .text : {
    /* Pull in all symbols in input sections named .text */
    *(.text)
    /* Do the same for sections starting with .text.,
       such as .text.foo */
    *(.text.*)
  }

  /* Do the same for ".bss", ".rodata", and ".data". */
  . = 0x5100000;
  .rodata : { *(.rodata); *(.rodata.*) }
  .data : { *(.data); *(.data.*) }
  .bss : { *(.bss); *(.bss.*) }
}