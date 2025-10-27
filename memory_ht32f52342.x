/* Memory layout for HT32F52342 */
MEMORY
{
  FLASH : ORIGIN = 0x00000000, LENGTH = 64K
  RAM   : ORIGIN = 0x20000000, LENGTH = 8K
}

/* Provide stack size (adjust based on your needs) */
_stack_size = 2K;

/* Place defmt data in flash (INFO sections don't occupy memory at runtime) */
SECTIONS {
  .defmt : {
    *(.defmt .defmt.*)
  } > FLASH
}