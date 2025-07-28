use embedded_hal::{digital::OutputPin, spi::SpiDevice};
use embedded_hal_async::spi::SpiDevice as AsyncSpiDevice;

use super::{Interface, InterfaceAsync, InterfaceItrAsync, InterfaceKind};

/// Spi interface error
#[derive(Clone, Copy, Debug)]
pub enum SpiError<SPI, DC> {
    /// SPI bus error
    Spi(SPI),
    /// Data/command pin error
    Dc(DC),
}

/// Spi interface, including a buffer
///
/// The buffer is used to gather batches of pixel data to be sent over SPI.
/// Larger buffers will generally be faster (with diminishing returns), at the expense of using more RAM.
/// The buffer should be at least big enough to hold a few pixels of data.
///
/// You may want to use [static_cell](https://crates.io/crates/static_cell)
/// to obtain a `&'static mut [u8; N]` buffer.
pub struct SpiInterface<'a, SPI, DC> {
    spi: SPI,
    dc: DC,
    buffer: &'a mut [u8],
}

/// Async SPI interface, including a buffer, using the async SPI API.
///
/// This variant allows the use of the memory-frugal, iterator-based approach
/// (vs the whole-framebuffer-at-once approach), while still using the async
/// API of the SPI bus driver.
///
/// On some platforms, such as STM32 with Embassy, DMA is not available for the
/// blocking API, only the async API. This makes display refresh too slow on
/// larger displays. On the other hand, such larger displays require too much
/// RAM to keep a full-screen framebuffer available for rendering.
///
/// This hybrid approach helps to overcome that issue.
pub struct SpiInterfaceItrAsync<'a, SPI, DC> {
    spi: SPI,
    dc: DC,
    buffer: &'a mut [u8],
}

/// Async version of above, framebuffer-based: TODO docs
pub struct SpiInterfaceAsync<SPI, DC> {
    spi: SPI,
    dc: DC,
}

impl<'a, SPI, DC> SpiInterface<'a, SPI, DC>
where
    SPI: SpiDevice,
    DC: OutputPin,
{
    /// Create new interface
    pub fn new(spi: SPI, dc: DC, buffer: &'a mut [u8]) -> Self {
        Self { spi, dc, buffer }
    }

    /// Release the DC pin and SPI peripheral back, deconstructing the interface
    pub fn release(self) -> (SPI, DC) {
        (self.spi, self.dc)
    }
}

impl<'a, SPI, DC> SpiInterfaceItrAsync<'a, SPI, DC>
where
    SPI: AsyncSpiDevice,
    DC: OutputPin,
{
    /// Create new interface
    pub fn new(spi: SPI, dc: DC, buffer: &'a mut [u8]) -> Self {
        Self { spi, dc, buffer }
    }

    /// Release the DC pin and SPI peripheral back, deconstructing the interface
    pub fn release(self) -> (SPI, DC) {
        (self.spi, self.dc)
    }
}

impl<SPI, DC> SpiInterfaceAsync<SPI, DC>
where
    SPI: AsyncSpiDevice,
    DC: OutputPin,
{
    /// Create new interface
    pub fn new(spi: SPI, dc: DC) -> Self {
        Self { spi, dc }
    }

    /// Release the DC pin and SPI peripheral back, deconstructing the interface
    pub fn release(self) -> (SPI, DC) {
        (self.spi, self.dc)
    }
}

impl<SPI, DC> Interface for SpiInterface<'_, SPI, DC>
where
    SPI: SpiDevice,
    DC: OutputPin,
{
    type Word = u8;
    type Error = SpiError<SPI::Error, DC::Error>;

    const KIND: InterfaceKind = InterfaceKind::Serial4Line;

    fn send_command(&mut self, command: u8, args: &[u8]) -> Result<(), Self::Error> {
        self.dc.set_low().map_err(SpiError::Dc)?;
        self.spi.write(&[command]).map_err(SpiError::Spi)?;
        self.dc.set_high().map_err(SpiError::Dc)?;
        self.spi.write(args).map_err(SpiError::Spi)?;
        Ok(())
    }

    fn send_pixels<const N: usize>(
        &mut self,
        pixels: impl IntoIterator<Item = [Self::Word; N]>,
    ) -> Result<(), Self::Error> {
        let mut arrays = pixels.into_iter();

        assert!(self.buffer.len() >= N);

        let mut done = false;
        while !done {
            let mut i = 0;
            for chunk in self.buffer.chunks_exact_mut(N) {
                if let Some(array) = arrays.next() {
                    let chunk: &mut [u8; N] = chunk.try_into().unwrap();
                    *chunk = array;
                    i += N;
                } else {
                    done = true;
                    break;
                };
            }
            self.spi.write(&self.buffer[..i]).map_err(SpiError::Spi)?;
        }
        Ok(())
    }

    fn send_repeated_pixel<const N: usize>(
        &mut self,
        pixel: [Self::Word; N],
        count: u32,
    ) -> Result<(), Self::Error> {
        let fill_count = core::cmp::min(count, (self.buffer.len() / N) as u32);
        let filled_len = fill_count as usize * N;
        for chunk in self.buffer[..(filled_len)].chunks_exact_mut(N) {
            let chunk: &mut [u8; N] = chunk.try_into().unwrap();
            *chunk = pixel;
        }

        let mut count = count;
        while count >= fill_count {
            self.spi
                .write(&self.buffer[..filled_len])
                .map_err(SpiError::Spi)?;
            count -= fill_count;
        }
        if count != 0 {
            self.spi
                .write(&self.buffer[..(count as usize * pixel.len())])
                .map_err(SpiError::Spi)?;
        }
        Ok(())
    }
}

impl<SPI, DC> InterfaceItrAsync for SpiInterfaceItrAsync<'_, SPI, DC>
where
    SPI: AsyncSpiDevice,
    DC: OutputPin,
{
    type Word = u8;
    type Error = SpiError<SPI::Error, DC::Error>;

    const KIND: InterfaceKind = InterfaceKind::Serial4Line;

    async fn send_command(&mut self, command: u8, args: &[u8]) -> Result<(), Self::Error> {
        self.dc.set_low().map_err(SpiError::Dc)?;
        self.spi.write(&[command]).await.map_err(SpiError::Spi)?;
        self.dc.set_high().map_err(SpiError::Dc)?;
        self.spi.write(args).await.map_err(SpiError::Spi)?;

        Ok(())
    }

    async fn send_pixels<const N: usize>(
        &mut self,
        pixels: impl IntoIterator<Item = [Self::Word; N]>,
    ) -> Result<(), Self::Error> {
        let mut arrays = pixels.into_iter();

        assert!(self.buffer.len() >= N);

        let mut done = false;
        while !done {
            let mut i = 0;
            for chunk in self.buffer.chunks_exact_mut(N) {
                if let Some(array) = arrays.next() {
                    let chunk: &mut [u8; N] = chunk.try_into().unwrap();
                    *chunk = array;
                    i += N;
                } else {
                    done = true;
                    break;
                };
            }
            self.spi
                .write(&self.buffer[..i])
                .await
                .map_err(SpiError::Spi)?;
        }
        Ok(())
    }

    async fn send_repeated_pixel<const N: usize>(
        &mut self,
        pixel: [Self::Word; N],
        count: u32,
    ) -> Result<(), Self::Error> {
        let fill_count = core::cmp::min(count, (self.buffer.len() / N) as u32);
        let filled_len = fill_count as usize * N;
        for chunk in self.buffer[..(filled_len)].chunks_exact_mut(N) {
            let chunk: &mut [u8; N] = chunk.try_into().unwrap();
            *chunk = pixel;
        }

        let mut count = count;
        while count >= fill_count {
            self.spi
                .write(&self.buffer[..filled_len])
                .await
                .map_err(SpiError::Spi)?;
            count -= fill_count;
        }
        if count != 0 {
            self.spi
                .write(&self.buffer[..(count as usize * pixel.len())])
                .await
                .map_err(SpiError::Spi)?;
        }
        Ok(())
    }
}

impl<SPI, DC> InterfaceAsync for SpiInterfaceAsync<SPI, DC>
where
    SPI: AsyncSpiDevice,
    DC: OutputPin,
{
    const KIND: InterfaceKind = InterfaceKind::Serial4Line;
    type Error = SpiError<SPI::Error, DC::Error>;
    type Word = u8;

    async fn send_command(&mut self, command: u8, args: &[u8]) -> Result<(), Self::Error> {
        self.dc.set_low().map_err(SpiError::Dc)?;
        self.spi.write(&[command]).await.map_err(SpiError::Spi)?;
        self.dc.set_high().map_err(SpiError::Dc)?;
        self.spi.write(args).await.map_err(SpiError::Spi)?;

        Ok(())
    }

    async fn send_buffer(&mut self, buf: &[u8]) -> Result<(), Self::Error> {
        self.spi.write(buf).await.map_err(SpiError::Spi)
    }
}
