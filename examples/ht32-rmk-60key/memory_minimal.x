/* Memory layout for HT32F52352 C18 revision - full RMK configuration */
MEMORY
{
  /* The final 512-byte physical page contains Option Bytes. */
  FLASH : ORIGIN = 0x00000000, LENGTH = 130560
  RAM   : ORIGIN = 0x20000000, LENGTH = 16K
}

/* Comfortable stack size for full RMK functionality */
_stack_size = 2K;

/* Adequate heap for dynamic allocations */
_heap_size = 1K;

/* Remove defmt sections completely to save space */
