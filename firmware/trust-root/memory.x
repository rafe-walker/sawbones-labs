/* RP2350 memory map for the trust-root image (#114). Flash size is the
 * smallest the board may carry; RAM is the striped SRAM0 to SRAM7. */
MEMORY {
    FLASH : ORIGIN = 0x10000000, LENGTH = 2048K
    RAM   : ORIGIN = 0x20000000, LENGTH = 512K
}

/* The boot ROM looks for an IMAGE_DEF block in the first 4 KiB of flash.
 * Put it straight after the vector table. */
SECTIONS {
    .start_block : ALIGN(4)
    {
        __start_block_addr = .;
        KEEP(*(.start_block));
    } > FLASH
} INSERT AFTER .vector_table;

/* Code starts after the block. */
_stext = ADDR(.start_block) + SIZEOF(.start_block);
