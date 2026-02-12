//! Internal PIO interrupt bindings and PIO-to-IRQ trait mappings.

#![cfg(not(feature = "host"))]

use embassy_rp::interrupt::typelevel::Binding;
use embassy_rp::pio::{Instance, InterruptHandler};

/// Trait mapping a PIO peripheral to its interrupt binding.
#[doc(hidden)] // Internal bound reused across device modules.
pub trait PioIrqMap: Instance {
    /// Interrupt binding type for this PIO resource.
    type Irqs: Binding<Self::Interrupt, InterruptHandler<Self>>;

    /// Returns interrupt bindings for this PIO resource.
    fn irqs() -> Self::Irqs;
}

::embassy_rp::bind_interrupts! {
    pub struct Pio0Irqs {
        PIO0_IRQ_0 => ::embassy_rp::pio::InterruptHandler<::embassy_rp::peripherals::PIO0>;
    }
}

impl PioIrqMap for embassy_rp::peripherals::PIO0 {
    type Irqs = Pio0Irqs;

    fn irqs() -> Self::Irqs {
        Pio0Irqs
    }
}

::embassy_rp::bind_interrupts! {
    pub struct Pio1Irqs {
        PIO1_IRQ_0 => ::embassy_rp::pio::InterruptHandler<::embassy_rp::peripherals::PIO1>;
    }
}

impl PioIrqMap for embassy_rp::peripherals::PIO1 {
    type Irqs = Pio1Irqs;

    fn irqs() -> Self::Irqs {
        Pio1Irqs
    }
}

#[cfg(feature = "pico2")]
::embassy_rp::bind_interrupts! {
    pub struct Pio2Irqs {
        PIO2_IRQ_0 => ::embassy_rp::pio::InterruptHandler<::embassy_rp::peripherals::PIO2>;
    }
}

#[cfg(feature = "pico2")]
impl PioIrqMap for embassy_rp::peripherals::PIO2 {
    type Irqs = Pio2Irqs;

    fn irqs() -> Self::Irqs {
        Pio2Irqs
    }
}
