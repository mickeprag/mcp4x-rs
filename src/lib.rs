//! This is a platform-agnostic Rust driver for the MCP41xxx and MCP42xxx SPI
//! digital potentiometers (digipot), based on the [`embedded-hal`] traits.
//!
//! [`embedded-hal`]: https://github.com/rust-embedded/embedded-hal
//!
//! This driver allows you to:
//! - Set a channel to a position.
//! - Shutdown a channel.
//!
//! ## The devices
//! The MCP41XXX and MCP42XXX devices are 256-position, digital potentiometers
//! available in 10 kΩ, 50 kΩ and 100 kΩ resistance versions. The MCP41XXX is
//! a single-channel device and is offered in an 8-pin PDIP or SOIC package.
//! The MCP42XXX contains two independent channels in a 14-pin PDIP, SOIC or
//! TSSOP package. The wiper position of the MCP41XXX/42XXX varies linearly
//! and is controlled via an industry-standard SPI interface.
//!
//! The devices consume <1 μA during static operation. A software shutdown
//! feature is provided that disconnects the "A" terminal from the resistor
//! stack and simultaneously connects the wiper to the "B" terminal.
//! In addition, the dual MCP42XXX has a SHDN pin that performs the same
//! function in hardware. During shutdown mode, the contents of the wiper
//! register can be changed and the potentiometer returns from shutdown to the
//! new value. The wiper is reset to the mid-scale position (80h) upon
//! power-up. The RS (reset) pin implements a hardware reset and also returns
//! the wiper to mid-scale.
//!
//! The MCP42XXX SPI interface includes both the SI and SO pins, allowing
//! daisy-chaining of multiple devices. Channel-to-channel resistance matching
//! on the MCP42XXX varies by less than 1%.
//!
//! Datasheet:
//! - [MCP41XXX/MCP42XXX](http://ww1.microchip.com/downloads/en/DeviceDoc/11195c.pdf)
//!

#![deny(unsafe_code)]
#![deny(missing_docs)]
// TODO #![deny(warnings)]
#![no_std]

use core::marker::PhantomData;
extern crate embedded_hal as hal;

/// All possible errors in this crate
#[derive(Debug)]
pub enum Error<E> {
    /// Communication error
    Comm(E),
}

/// Channel selector
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Channel {
    /// Channel 0
    Ch0,
    /// Channel 1 (only for MCP42XXX devices)
    Ch1,
}

impl Channel {
    fn get_bits(self) -> u8 {
        match self {
            Channel::Ch0 => 1,
            Channel::Ch1 => 2,
        }
    }
}

enum Command {
    /// Set a channel to a position
    SetPosition(Channel, u8),
    /// Shutdown channel
    Shutdown(Channel),
}

impl Command {
    fn get_command_byte(&self) -> u8 {
        match *self {
            Command::SetPosition(channel, _) => 0b0001_0000 | channel.get_bits(),
            Command::Shutdown(channel) => 0b0010_0000 | channel.get_bits(),
        }
    }
    fn get_data_byte(&self) -> u8 {
        match *self {
            Command::SetPosition(_, position) => position,
            Command::Shutdown(_) => 0,
        }
    }
}

/// IC markers
pub mod ic {
    /// MCP41x IC marker
    pub struct Mcp41x(());
}

/// MCP4x digital potentiometer driver
#[derive(Debug, Default)]
pub struct Mcp4x<DI, IC> {
    iface: DI,
    _ic: PhantomData<IC>,
}

impl<DI, IC, E> Mcp4x<DI, IC>
where
    DI: interface::WriteCommand<Error = E>
{
    /// Set a channel to a position
    pub fn set_position(&mut self, channel: Channel, position: u8) -> Result<(), Error<E>> {
        // TODO check channel is appropriate for IC
        let cmd = Command::SetPosition(channel, position);
        self.iface.write_command(cmd.get_command_byte(), cmd.get_data_byte())
    }

    /// Shutdown a channel
    pub fn shutdown(&mut self, channel: Channel) -> Result<(), Error<E>> {
        // TODO check channel is appropriate for IC
        let cmd = Command::Shutdown(channel);
        self.iface.write_command(cmd.get_command_byte(), cmd.get_data_byte())
    }
}

impl<SPI, CS> Mcp4x<interface::SpiInterface<SPI, CS>, ic::Mcp41x> {
    /// Create new MCP41x device instance
    pub fn new_mcp41x(spi: SPI, chip_select: CS) -> Self {
        Mcp4x {
            iface: interface::SpiInterface {
                spi,
                cs: chip_select
            },
            _ic: PhantomData,
        }
    }

    /// Destroy driver instance, return SPI bus instance and CS output pin.
    pub fn destroy_mcp41x(self) -> (SPI, CS) {
        (self.iface.spi, self.iface.cs)
    }
}

#[doc(hidden)]
pub mod interface;

mod private {
    use super::interface;
    pub trait Sealed {}

    impl<SPI, CS> Sealed for interface::SpiInterface<SPI, CS> {}
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! set_position_cmd {
        ($name:ident, $channel:ident, $position:expr, $expected_cmd:expr) => {
            #[test]
            fn $name() {
                let cmd = Command::SetPosition(Channel::$channel, $position);
                assert_eq!($expected_cmd, cmd.get_command_byte());
                assert_eq!($position, cmd.get_data_byte());
            }
        };
    }

    set_position_cmd!(can_set_position_ch0, Ch0, 127, 0b0001_0001);
    set_position_cmd!(can_set_position_ch1, Ch1, 127, 0b0001_0010);

    macro_rules! shutdown {
        ($name:ident, $channel:ident, $expected_cmd:expr) => {
            #[test]
            fn $name() {
                let cmd = Command::Shutdown(Channel::$channel);
                assert_eq!($expected_cmd, cmd.get_command_byte());
                assert_eq!(0, cmd.get_data_byte());
            }
        };
    }

    shutdown!(can_shutdown_ch_0, Ch0, 0b0010_0001);
    shutdown!(can_shutdown_ch_1, Ch1, 0b0010_0010);
}