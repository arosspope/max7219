extern crate embedded_hal;

use embedded_hal::blocking::spi::Write;
use embedded_hal::digital::v2::OutputPin;

use crate::{Command, PinError, MAX_DISPLAYS};

/// Describes the interface used to connect to the MX7219
pub trait Connector {
    fn devices(&self) -> usize;

    ///
    /// Writes data to given register as described by command
    ///
    /// # Arguments
    ///
    /// * `addr` - display to address as connected in series (0 -> last)
    /// * `command` - the command/register on the display to write to
    /// * `data` - the data byte value to write
    ///
    /// # Errors
    ///
    /// * `PinError` - returned in case there was an error setting a PIN on the device
    ///
    fn write_data(&mut self, addr: usize, command: Command, data: u8) -> Result<(), PinError> {
        self.write_raw(addr, command as u8, data)
    }

    ///
    /// Writes data to given register as described by command
    ///
    /// # Arguments
    ///
    /// * `addr` - display to address as connected in series (0 -> last)
    /// * `header` - the command/register on the display to write to as u8
    /// * `data` - the data byte value to write
    ///
    /// # Errors
    ///
    /// * `PinError` - returned in case there was an error setting a PIN on the device
    ///
    fn write_raw(&mut self, addr: usize, header: u8, data: u8) -> Result<(), PinError>;
}

/// Direct GPIO pins connector
pub struct PinConnector<DATA, CS, SCK>
where
    DATA: OutputPin,
    CS: OutputPin,
    SCK: OutputPin,
{
    devices: usize,
    buffer: [u8; MAX_DISPLAYS * 2],
    data: DATA,
    cs: CS,
    sck: SCK,
}

impl<DATA, CS, SCK> PinConnector<DATA, CS, SCK>
where
    DATA: OutputPin,
    CS: OutputPin,
    SCK: OutputPin,
{
    pub(crate) fn new(displays: usize, data: DATA, cs: CS, sck: SCK) -> Self {
        PinConnector {
            devices: displays,
            buffer: [0; MAX_DISPLAYS * 2],
            data,
            cs,
            sck,
        }
    }
}

impl<DATA, CS, SCK> Connector for PinConnector<DATA, CS, SCK>
where
    DATA: OutputPin,
    CS: OutputPin,
    SCK: OutputPin,
    PinError: core::convert::From<DATA::Error>,
    PinError: core::convert::From<CS::Error>,
    PinError: core::convert::From<SCK::Error>,
{
    fn devices(&self) -> usize {
        self.devices
    }

    fn write_raw(&mut self, addr: usize, header: u8, data: u8) -> Result<(), PinError> {
        let offset = addr * 2;
        let max_bytes = self.devices * 2;
        self.buffer = [0; MAX_DISPLAYS * 2];

        self.buffer[offset] = header;
        self.buffer[offset + 1] = data;

        self.cs.set_low()?;
        for b in 0..max_bytes {
            let value = self.buffer[b];

            for i in 0..8 {
                if value & (1 << (7 - i)) > 0 {
                    self.data.set_high()?;
                } else {
                    self.data.set_low()?;
                }

                self.sck.set_high()?;
                self.sck.set_low()?;
            }
        }
        self.cs.set_high()?;

        Ok(())
    }
}

pub struct SpiConnector<SPI>
where
    SPI: Write<u8>,
{
    devices: usize,
    buffer: [u8; MAX_DISPLAYS * 2],
    spi: SPI,
}

/// Hardware controlled CS connector with SPI transfer
impl<SPI> SpiConnector<SPI>
where
    SPI: Write<u8>,
{
    pub(crate) fn new(displays: usize, spi: SPI) -> Self {
        SpiConnector {
            devices: displays,
            buffer: [0; MAX_DISPLAYS * 2],
            spi,
        }
    }
}

impl<SPI> Connector for SpiConnector<SPI>
where
    SPI: Write<u8>,
    PinError: core::convert::From<SPI::Error>,
{
    fn devices(&self) -> usize {
        self.devices
    }

    fn write_raw(&mut self, addr: usize, header: u8, data: u8) -> Result<(), PinError> {
        let offset = addr * 2;
        let max_bytes = self.devices * 2;
        self.buffer = [0; MAX_DISPLAYS * 2];

        self.buffer[offset] = header;
        self.buffer[offset + 1] = data;

        self.spi.write(&self.buffer[0..max_bytes])?;

        Ok(())
    }
}

/// Software controlled CS connector with SPI transfer
pub struct SpiConnectorSW<SPI, CS>
where
    SPI: Write<u8>,
    CS: OutputPin,
{
    spi_c: SpiConnector<SPI>,
    cs: CS,
}

impl<SPI, CS> SpiConnectorSW<SPI, CS>
where
    SPI: Write<u8>,
    CS: OutputPin,
{
    pub(crate) fn new(displays: usize, spi: SPI, cs: CS) -> Self {
        SpiConnectorSW {
            spi_c: SpiConnector::new(displays, spi),
            cs,
        }
    }
}

impl<SPI, CS> Connector for SpiConnectorSW<SPI, CS>
where
    SPI: Write<u8>,
    CS: OutputPin,
    PinError: core::convert::From<SPI::Error>,
    PinError: core::convert::From<CS::Error>,
{
    fn devices(&self) -> usize {
        self.spi_c.devices
    }

    fn write_raw(&mut self, addr: usize, header: u8, data: u8) -> Result<(), PinError> {
        self.cs.set_low()?;
        self.spi_c.write_raw(addr, header, data)?;
        self.cs.set_high()?;

        Ok(())
    }
}
