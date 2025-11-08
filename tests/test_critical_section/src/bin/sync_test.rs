//! Synchronous test for critical section implementation (no Embassy executor)

#![no_std]
#![no_main]

use embassy_ht32f523xx as hal;
use {defmt_rtt as _, panic_probe as _};
use cortex_m_rt::entry;

// Shared data for testing critical section protection
static mut SHARED_COUNTER: u32 = 0;
static mut SHARED_ARRAY: [u32; 8] = [0; 8];

#[entry]
fn main() -> ! {
    let _p = hal::init(hal::Config::default());

    defmt::info!("Starting synchronous critical section tests");

    // Test 1: Basic critical section functionality
    test_basic_critical_section();

    // Test 2: Nested critical section safety
    test_nested_critical_sections();

    // Test 3: Atomic operations within critical sections
    test_atomic_operations();

    defmt::info!("✅ All critical section tests passed successfully");

    loop {
        cortex_m::asm::nop();
    }
}

fn test_basic_critical_section() {
    defmt::info!("Testing basic critical section functionality");

    // Test simple protection of shared data
    for i in 0..10 {
        critical_section::with(|_| {
            unsafe {
                SHARED_COUNTER = i;
            }
        });
    }

    // Verify final value
    let final_value = critical_section::with(|_| {
        unsafe { SHARED_COUNTER }
    });

    defmt::info!("Basic test - final counter value: {}", final_value);
    assert_eq!(final_value, 9, "Basic critical section test failed");
}

fn test_nested_critical_sections() {
    defmt::info!("Testing nested critical sections");

    let mut outer_value = 0u32;

    // Test nested critical sections
    critical_section::with(|_| {
        outer_value = 100;

        critical_section::with(|_| {
            unsafe {
                SHARED_COUNTER = outer_value;
                SHARED_ARRAY[0] = 200;
            }
        });

        outer_value = 150;
    });

    // Verify values are consistent
    let (shared_counter, array_value) = critical_section::with(|_| {
        unsafe { (SHARED_COUNTER, SHARED_ARRAY[0]) }
    });

    defmt::info!("Nested test - counter: {}, array[0]: {}", shared_counter, array_value);
    assert_eq!(shared_counter, 100, "Nested critical section counter test failed");
    assert_eq!(array_value, 200, "Nested critical section array test failed");
}

fn test_atomic_operations() {
    defmt::info!("Testing atomic operations within critical sections");

    // Test multiple operations in one critical section
    critical_section::with(|_| {
        unsafe {
            // Clear array
            for i in 0..8 {
                SHARED_ARRAY[i] = 0;
            }

            // Perform multiple operations
            SHARED_ARRAY[0] = 1;
            SHARED_ARRAY[1] = SHARED_ARRAY[0] + 1;
            SHARED_ARRAY[2] = SHARED_ARRAY[1] * 2;
            SHARED_ARRAY[3] = SHARED_ARRAY[2] + SHARED_ARRAY[0];

            // Update counter
            SHARED_COUNTER = 42;
        }
    });

    // Verify all operations completed correctly
    let (counter, array_values) = critical_section::with(|_| {
        unsafe {
            let mut values = [0u32; 4];
            for i in 0..4 {
                values[i] = SHARED_ARRAY[i];
            }
            (SHARED_COUNTER, values)
        }
    });

    defmt::info!("Atomic test - counter: {}, array: [{}, {}, {}, {}]",
        counter, array_values[0], array_values[1], array_values[2], array_values[3]);
    assert_eq!(counter, 42, "Atomic test counter failed");
    assert_eq!(array_values[0], 1, "Atomic test array[0] failed");
    assert_eq!(array_values[1], 2, "Atomic test array[1] failed");
    assert_eq!(array_values[2], 4, "Atomic test array[2] failed");
    assert_eq!(array_values[3], 5, "Atomic test array[3] failed");
}