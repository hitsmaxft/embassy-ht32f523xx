/* Memory layout for HT32F52352. The final 512-byte page contains Option Bytes. */
MEMORY
{
  FLASH : ORIGIN = 0x00000000, LENGTH = 130560
  RAM   : ORIGIN = 0x20000000, LENGTH = 16K
}
