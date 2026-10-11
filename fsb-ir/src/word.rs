#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct Word(u32);

impl Word {
    pub const fn from_bits(bits: u32) -> Self {
        Self(bits)
    }

    pub const fn from_i32(value: i32) -> Self {
        Self(value.cast_unsigned())
    }

    pub const fn from_f32(value: f32) -> Self {
        Self(value.to_bits())
    }

    pub const fn bits(self) -> u32 {
        self.0
    }
}
