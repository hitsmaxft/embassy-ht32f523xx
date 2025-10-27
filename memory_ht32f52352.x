/* Memory layout for HT32F52352 */
MEMORY
{
  FLASH : ORIGIN = 0x00000000, LENGTH = 128K
  RAM   : ORIGIN = 0x20000000, LENGTH = 16K
}

/* Provide stack size (adjust based on your needs) */
_stack_size = 4K;

/* Place defmt data in flash (INFO sections don't occupy memory at runtime) */
SECTIONS {
  .defmt : {
    *(.defmt .defmt.*)
  } > FLASH
}