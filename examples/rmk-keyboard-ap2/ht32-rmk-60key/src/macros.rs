use embassy_ht32::hal::gpio::{Pin, Input, Output};
use embedded_hal::digital::OutputPin;

macro_rules! config_matrix_pins_ht32 {
    (gpio: $gpio:ident, input: [$($in_pin:ident), *], output: [$($out_pin:ident), +]) => {
        {
            let mut output_pins = [$($gpio.$out_pin.into_push_pull_output()), +];
            let input_pins = [$($gpio.$in_pin.into_floating_input()), +];
            output_pins.iter_mut().for_each(|p| {
                let _ = p.set_low();
            });
            (input_pins, output_pins)
        }
    };
}