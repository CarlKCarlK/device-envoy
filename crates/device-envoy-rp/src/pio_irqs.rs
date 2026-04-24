//! Internal PIO interrupt bindings and PIO-to-IRQ trait mappings.

#![cfg(not(feature = "host"))]

use embassy_rp::interrupt::typelevel::Binding;
use embassy_rp::pio::{Instance, InterruptHandler};
use embassy_rp::{dma, peripherals};

/// Trait mapping a PIO peripheral to its interrupt binding.
#[doc(hidden)] // Internal bound reused across device modules.
pub trait PioIrqMap: Instance {
    /// Interrupt binding type for this PIO resource.
    type Irqs: Binding<Self::Interrupt, InterruptHandler<Self>>;

    /// Returns interrupt bindings for this PIO resource.
    fn irqs() -> Self::Irqs;
}

/// Trait mapping DMA channels to their interrupt binding.
#[doc(hidden)] // Internal bound reused across device modules.
pub trait DmaIrqMap: dma::ChannelInstance {
    /// Interrupt binding type for this DMA channel.
    type Irqs: Binding<Self::Interrupt, dma::InterruptHandler<Self>>;

    /// Returns interrupt bindings for this DMA channel.
    fn irqs() -> Self::Irqs;
}

::embassy_rp::bind_interrupts! {
    pub struct Pio0Irqs {
        PIO0_IRQ_0 => ::embassy_rp::pio::InterruptHandler<::embassy_rp::peripherals::PIO0>;
    }
}

macro_rules! impl_dma_irq_map_all_irqs {
    ($($dma_channel:ty),+ $(,)?) => {
        $(
            impl DmaIrqMap for $dma_channel {
                type Irqs = DmaAllIrqs;

                fn irqs() -> Self::Irqs {
                    DmaAllIrqs
                }
            }
        )+
    };
}

impl_dma_irq_map_all_irqs!(
    peripherals::DMA_CH0,
    peripherals::DMA_CH1,
    peripherals::DMA_CH2,
    peripherals::DMA_CH3,
    peripherals::DMA_CH4,
    peripherals::DMA_CH5,
    peripherals::DMA_CH6,
    peripherals::DMA_CH7,
    peripherals::DMA_CH8,
    peripherals::DMA_CH9,
    peripherals::DMA_CH10,
    peripherals::DMA_CH11,
);

#[cfg(feature = "pico2")]
impl_dma_irq_map_all_irqs!(
    peripherals::DMA_CH12,
    peripherals::DMA_CH13,
    peripherals::DMA_CH14,
    peripherals::DMA_CH15,
);

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

::embassy_rp::bind_interrupts! {
    pub struct DmaAllIrqs {
        DMA_IRQ_0 =>
            ::embassy_rp::dma::InterruptHandler<::embassy_rp::peripherals::DMA_CH0>,
            ::embassy_rp::dma::InterruptHandler<::embassy_rp::peripherals::DMA_CH1>,
            ::embassy_rp::dma::InterruptHandler<::embassy_rp::peripherals::DMA_CH2>,
            ::embassy_rp::dma::InterruptHandler<::embassy_rp::peripherals::DMA_CH3>,
            ::embassy_rp::dma::InterruptHandler<::embassy_rp::peripherals::DMA_CH4>,
            ::embassy_rp::dma::InterruptHandler<::embassy_rp::peripherals::DMA_CH5>,
            ::embassy_rp::dma::InterruptHandler<::embassy_rp::peripherals::DMA_CH6>,
            ::embassy_rp::dma::InterruptHandler<::embassy_rp::peripherals::DMA_CH7>,
            ::embassy_rp::dma::InterruptHandler<::embassy_rp::peripherals::DMA_CH8>,
            ::embassy_rp::dma::InterruptHandler<::embassy_rp::peripherals::DMA_CH9>,
            ::embassy_rp::dma::InterruptHandler<::embassy_rp::peripherals::DMA_CH10>,
            ::embassy_rp::dma::InterruptHandler<::embassy_rp::peripherals::DMA_CH11>,
            #[cfg(feature = "pico2")]
            ::embassy_rp::dma::InterruptHandler<::embassy_rp::peripherals::DMA_CH12>,
            #[cfg(feature = "pico2")]
            ::embassy_rp::dma::InterruptHandler<::embassy_rp::peripherals::DMA_CH13>,
            #[cfg(feature = "pico2")]
            ::embassy_rp::dma::InterruptHandler<::embassy_rp::peripherals::DMA_CH14>,
            #[cfg(feature = "pico2")]
            ::embassy_rp::dma::InterruptHandler<::embassy_rp::peripherals::DMA_CH15>;
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
