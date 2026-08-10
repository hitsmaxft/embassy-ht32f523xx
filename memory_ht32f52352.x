/* Memory layout for HT32F52352 */
MEMORY
{
  /* The final 512-byte physical page contains Option Bytes. */
  FLASH : ORIGIN = 0x00000000, LENGTH = 130560
  RAM   : ORIGIN = 0x20000000, LENGTH = 16K
}
