//! Display models.

use crate::{
    dcs::{self, SetAddressMode},
    init_engine::InitEngine,
    options::{self, ModelOptions, Rotation},
    ConfigurationError,
};
use embedded_graphics_core::prelude::RgbColor;
use embedded_hal::delay::DelayNs;

// existing model implementations
// mod gc9107;
// mod gc9a01;
// mod ili9341;
// mod ili9342c;
// mod ili934x;
// mod ili9486;
// mod ili9488;
// mod ili948x;
// mod rm67162;
// mod st7735s;
mod st7789;
mod st7796;

// pub use gc9107::*;
// pub use gc9a01::*;
// pub use ili9341::*;
// pub use ili9342c::*;
// pub use ili9486::*;
// pub use ili9488::*;
// pub use rm67162::*;
// pub use st7735s::*;
pub use st7789::*;
pub use st7796::*;

/// Display model.
pub trait Model {
    /// The color format.
    type ColorFormat: RgbColor;

    /// The framebuffer size in pixels.
    const FRAMEBUFFER_SIZE: (u16, u16);

    /// Duration of the active low reset pulse in µs.
    const RESET_DURATION: u32 = 10;

    /// Initializes the display for this model with MADCTL from [crate::Display]
    /// and returns the value of MADCTL set by init
    fn init<IE>(
        &mut self,
        options: &ModelOptions,
        ie: &mut IE,
    ) -> Result<SetAddressMode, ModelInitError<IE::Error>>
    where
        IE: InitEngine;

    /// Updates the address window of the display.
    fn update_address_window<IE>(
        ie: &mut IE,
        _rotation: Rotation,
        sx: u16,
        sy: u16,
        ex: u16,
        ey: u16,
    ) -> Result<(), IE::Error>
    where
        IE: InitEngine,
    {
        ie.queue_command(dcs::SetColumnAddress::new(sx, ex))?;
        ie.queue_command(dcs::SetPageAddress::new(sy, ey))
    }

    ///
    /// Need to call [Self::wake] before issuing other commands
    ///
    fn sleep<IE, DELAY>(ie: &mut IE, _delay: &mut DELAY) -> Result<(), IE::Error>
    where
        IE: InitEngine,
        DELAY: DelayNs,
    {
        ie.queue_command(dcs::EnterSleepMode)?;
        // All supported models requires a 120ms delay before issuing other commands
        ie.queue_delay_us(120_000)?;
        Ok(())
    }
    ///
    /// Wakes the display after it's been set to sleep via [Self::sleep]
    ///
    fn wake<IE, DELAY>(ie: &mut IE, _delay: &mut DELAY) -> Result<(), IE::Error>
    where
        IE: InitEngine,
        DELAY: DelayNs,
    {
        ie.queue_command(dcs::ExitSleepMode)?;
        // ST7789 and st7735s have the highest minimal delay of 120ms
        ie.queue_delay_us(120_000)?;
        Ok(())
    }
    ///
    /// We need WriteMemoryStart before write pixel
    ///
    fn write_memory_start<IE>(ie: &mut IE) -> Result<(), IE::Error>
    where
        IE: InitEngine,
    {
        ie.queue_command(dcs::WriteMemoryStart)
    }
    ///
    /// SoftReset
    ///
    fn software_reset<IE>(ie: &mut IE) -> Result<(), IE::Error>
    where
        IE: InitEngine,
    {
        ie.queue_command(dcs::SoftReset)
    }
    ///
    /// This function will been called if user update options
    ///
    fn update_options<IE>(&self, ie: &mut IE, options: &ModelOptions) -> Result<(), IE::Error>
    where
        IE: InitEngine,
    {
        let madctl = SetAddressMode::from(options);
        ie.queue_command(madctl)
    }

    ///
    /// Configures the tearing effect output.
    ///
    fn set_tearing_effect<IE>(
        ie: &mut IE,
        tearing_effect: options::TearingEffect,
        _options: &ModelOptions,
    ) -> Result<(), IE::Error>
    where
        IE: InitEngine,
    {
        ie.queue_command(dcs::SetTearingEffect::new(tearing_effect))
    }

    /// Sets the vertical scroll region.
    ///
    /// The `top_fixed_area` and `bottom_fixed_area` arguments can be used to
    /// define an area on the top and/or bottom of the display which won't be
    /// affected by scrolling.
    ///
    /// Note that this method is not affected by the current display orientation
    /// and will always scroll vertically relative to the default display
    /// orientation.
    ///
    /// The combined height of the fixed area must not larger than the
    /// height of the framebuffer height in the default orientation.
    ///
    /// After the scrolling region is defined the [`set_vertical_scroll_offset`](Self::set_vertical_scroll_offset) can be
    /// used to scroll the display.
    fn set_vertical_scroll_region<IE>(
        ie: &mut IE,
        top_fixed_area: u16,
        bottom_fixed_area: u16,
    ) -> Result<(), IE::Error>
    where
        IE: InitEngine,
    {
        let rows = Self::FRAMEBUFFER_SIZE.1;

        let vscrdef = if top_fixed_area + bottom_fixed_area > rows {
            dcs::SetScrollArea::new(rows, 0, 0)
        } else {
            dcs::SetScrollArea::new(
                top_fixed_area,
                rows - top_fixed_area - bottom_fixed_area,
                bottom_fixed_area,
            )
        };

        ie.queue_command(vscrdef)
    }

    /// Sets the vertical scroll offset.
    ///
    /// Setting the vertical scroll offset shifts the vertical scroll region
    /// upwards by `offset` pixels.
    ///
    /// Use [`set_vertical_scroll_region`](Self::set_vertical_scroll_region) to setup the scroll region, before
    /// using this method.
    fn set_vertical_scroll_offset<IE>(ie: &mut IE, offset: u16) -> Result<(), IE::Error>
    where
        IE: InitEngine,
    {
        let vscad = dcs::SetScrollStart::new(offset);
        ie.queue_command(vscad)
    }
}

/// Error returned by [`Model::init`].
///
/// This error type is used internally by implementations of the [`Model`]
/// trait.
#[derive(Debug)]
pub enum ModelInitError<DiError> {
    /// Error caused by the display interface.
    Interface(DiError),

    /// The init engine's queue, used for this Model's init, was too small
    InitEngineQueueFull,

    /// Invalid configuration error.
    ///
    /// This error is returned when the configuration passed to the builder is
    /// invalid. For example, when the combination of bit depth and interface
    /// kind isn't supported by the selected model.
    InvalidConfiguration(ConfigurationError),
}

impl<DiError> From<DiError> for ModelInitError<DiError> {
    fn from(value: DiError) -> Self {
        Self::Interface(value)
    }
}

// #[cfg(test)]
// mod tests {
//     use embedded_graphics::pixelcolor::Rgb565;

//     use crate::{
//         Builder,
//         _mock::{MockDelay, MockDisplayInterface},
//         interface::InterfaceKind,
//         ConfigurationError, InitError,
//     };

//     use super::*;

//     struct OnlyOneKindModel(InterfaceKind);

//     impl Model for OnlyOneKindModel {
//         type ColorFormat = Rgb565;

//         const FRAMEBUFFER_SIZE: (u16, u16) = (16, 16);

//         fn init<DELAY, DI>(
//             &mut self,
//             _di: &mut DI,
//             _delay: &mut DELAY,
//             _options: &ModelOptions,
//         ) -> Result<SetAddressMode, ModelInitError<DI::Error>>
//         where
//             DELAY: DelayNs,
//             DI: Interface,
//         {
//             if DI::KIND != self.0 {
//                 return Err(ModelInitError::InvalidConfiguration(
//                     ConfigurationError::UnsupportedInterface,
//                 ));
//             }

//             Ok(SetAddressMode::default())
//         }
//     }

//     #[test]
//     fn test_assert_interface_kind_serial() {
//         Builder::new(
//             OnlyOneKindModel(InterfaceKind::Serial4Line),
//             MockDisplayInterface,
//         )
//         .init(&mut MockDelay)
//         .unwrap();
//     }

//     #[test]
//     fn test_assert_interface_kind_parallel() {
//         assert!(matches!(
//             Builder::new(
//                 OnlyOneKindModel(InterfaceKind::Parallel8Bit),
//                 MockDisplayInterface,
//             )
//             .init(&mut MockDelay),
//             Err(InitError::InvalidConfiguration(
//                 ConfigurationError::UnsupportedInterface
//             ))
//         ));
//     }
// }
