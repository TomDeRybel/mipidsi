use embedded_graphics_core::pixelcolor::Rgb565;

use crate::{
    dcs::SetAddressMode,
    models::{Model, ModelInitError},
    options::ModelOptions,
};

use super::InitEngine;

/// ST7796 display in Rgb565 color mode.
pub struct ST7796;

impl Model for ST7796 {
    type ColorFormat = Rgb565;
    const FRAMEBUFFER_SIZE: (u16, u16) = (320, 480);

    fn init<IE>(
        &mut self,
        options: &ModelOptions,
        ie: &mut IE,
    ) -> Result<SetAddressMode, ModelInitError<IE::Error>>
    where
        IE: InitEngine,
    {
        // if !matches!(
        //     DI::KIND,
        //     InterfaceKind::Serial4Line | InterfaceKind::Parallel8Bit | InterfaceKind::Parallel16Bit
        // ) {
        //     return Err(ModelInitError::InvalidConfiguration(
        //         ConfigurationError::UnsupportedInterface,
        //     ));
        // }

        super::ST7789.init(options, ie)
    }
}
