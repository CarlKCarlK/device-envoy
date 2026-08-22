#[cfg(target_os = "none")]
use core::num::NonZeroU8;

#[cfg(target_os = "none")]
use esp_hal::gpio::{self, Level};

#[cfg(target_os = "none")]
use crate::Error::IndexOutOfBounds;
#[cfg(target_os = "none")]
use crate::Result;

/// Array of GPIO output pins for LED displays.
#[cfg(target_os = "none")]
pub struct OutputArray<'a, const N: usize>([gpio::Output<'a>; N]);

#[cfg(target_os = "none")]
impl<'a, const N: usize> OutputArray<'a, N> {
    /// Creates a new output array from GPIO pins.
    #[must_use]
    pub const fn new(outputs: [gpio::Output<'a>; N]) -> Self {
        Self(outputs)
    }

    #[inline]
    pub(crate) fn set_levels_at_indexes(&mut self, indexes: &[u8], level: Level) -> Result<()> {
        for &index in indexes {
            self.set_level_at_index(index, level)?;
        }
        Ok(())
    }

    #[inline]
    pub(crate) fn set_level_at_index(&mut self, index: u8, level: Level) -> Result<()> {
        self.get_mut(index as usize)
            .ok_or(IndexOutOfBounds)?
            .set_level(level);
        Ok(())
    }

    #[inline]
    pub(crate) fn get_mut(&mut self, index: usize) -> Option<&mut gpio::Output<'a>> {
        self.0.get_mut(index)
    }
}

#[cfg(target_os = "none")]
impl OutputArray<'_, { u8::BITS as usize }> {
    #[expect(clippy::shadow_reuse, reason = "Converting NonZeroU8 to u8")]
    #[inline]
    pub(crate) fn set_from_nonzero_bits(&mut self, bits: NonZeroU8) {
        let mut bits: u8 = bits.get();
        for output in &mut self.0 {
            let level: Level = ((bits & 1) == 1).into();
            output.set_level(level);
            bits >>= 1;
        }
    }
}
