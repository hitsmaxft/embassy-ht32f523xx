use rmk::action::KeyAction;
use rmk::{a, k, layer, mo};

pub(crate) const COL: usize = 14;
pub(crate) const ROW: usize = 5;
pub(crate) const NUM_LAYER: usize = 3;

#[rustfmt::skip]
pub const fn get_default_keymap() -> [[[KeyAction; COL]; ROW]; NUM_LAYER] {
    [
        // Layer 0: Base Layer (QWERTY)
        layer!([
            [k!(Escape),    k!(Kb1), k!(Kb2), k!(Kb3), k!(Kb4), k!(Kb5), k!(Kb6), k!(Kb7), k!(Kb8),    k!(Kb9),    k!(Kb0),    k!(Minus),    k!(Equal),     k!(Backspace)],
            [k!(Tab),       k!(Q),   k!(W),   k!(E),   k!(R),   k!(T),   k!(Y),   k!(U),   k!(I),      k!(O),      k!(P),      k!(LBracket), k!(RBracket),  k!(Backslash)],
            [mo!(1),        k!(A),   k!(S),   k!(D),   k!(F),   k!(G),   k!(H),   k!(J),   k!(K),      k!(L),      k!(Semicolon), k!(Quote),    k!(Enter),     a!(No)],
            [k!(LShift),    a!(No),  k!(Z),   k!(X),   k!(C),   k!(V),   k!(B),   k!(N),   k!(M),      k!(Comma),  k!(Dot),    k!(Slash),    k!(RShift),    a!(No)],
            [k!(LCtrl),     a!(No),  k!(LGui), k!(LAlt), a!(No), a!(No),  k!(Space), a!(No), a!(No),    k!(RAlt),   mo!(1),     mo!(2),       k!(RCtrl),     a!(No)]
        ]),
        // Layer 1: Function Layer
        layer!([
            [k!(Grave),     k!(F1),  k!(F2),  k!(F3),  k!(F4),  k!(F5),  k!(F6),  k!(F7),  k!(F8),     k!(F9),     k!(F10),    k!(F11),      k!(F12),       k!(Delete)],
            [a!(No),        a!(No),  k!(Up),  a!(No),  a!(No),  a!(No),  a!(No),  a!(No),  a!(No),     a!(No),     k!(PrintScreen), k!(Home),     k!(End),       a!(No)],
            [a!(No),        k!(Left), k!(Down), k!(Right), a!(No), a!(No),  a!(No),  a!(No),  a!(No),     a!(No),     k!(PageUp), k!(PageDown), a!(No),        a!(No)],
            [a!(No),        a!(No),  k!(AudioVolUp), k!(AudioVolDown), k!(Mute), a!(No),  a!(No),  a!(No),  a!(No),     a!(No),     k!(Insert), k!(Delete),   a!(No),        a!(No)],
            [a!(No),        a!(No),  a!(No),  a!(No),  a!(No),  a!(No),  a!(No),  a!(No),  a!(No),     a!(No),     a!(No),     mo!(2),       a!(No),        a!(No)]
        ]),
        // Layer 2: System Layer
        layer!([
            [a!(No),        a!(No),  a!(No),  a!(No),  a!(No),  a!(No),  a!(No),  a!(No),  a!(No),     a!(No),     a!(No),     a!(No),       a!(No),        a!(No)],
            [a!(No),        a!(No),  k!(Up),  a!(No),  a!(No),  a!(No),  a!(No),  a!(No),  a!(No),     a!(No),     k!(PrintScreen), k!(Home),     k!(End),       a!(No)],
            [a!(No),        k!(Left), k!(Down), k!(Right), a!(No), a!(No),  a!(No),  a!(No),  a!(No),     a!(No),     k!(PageUp), k!(PageDown), a!(No),        a!(No)],
            [a!(No),        a!(No),  a!(No),  a!(No),  a!(No),  a!(No),  a!(No),  a!(No),  a!(No),     a!(No),     k!(Insert), k!(Delete),   a!(No),        a!(No)],
            [a!(No),        a!(No),  a!(No),  a!(No),  a!(No),  a!(No),  a!(No),  a!(No),  a!(No),     a!(No),     a!(No),     a!(No),       a!(No),        a!(No)]
        ]),
    ]
}