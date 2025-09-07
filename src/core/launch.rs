use std::{error, fmt, string::FromUtf8Error};

use ectool::{Access, AccessHid, Ec, Error as EcError};
use hidapi::{HidApi, HidError};
use thiserror::Error;

#[derive(Debug)]
pub struct EcWrap(pub EcError);

impl fmt::Display for EcWrap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0) // Display via Debug
    }
}
impl error::Error for EcWrap {}

#[derive(Debug, Error)]
pub enum LaunchError {
    #[error("EC error: {0}")]
    Ec(#[from] EcWrap),
    #[error("HID error: {0}")]
    Hid(#[from] HidError),
    #[error("device not found")]
    DeviceNotFound,
    #[error("Unicode Error: {0}")]
    UnicodeError(#[from] FromUtf8Error),
    #[error("Unkown Led Mode: {0}")]
    UnknownLedMode(u8),
}

impl From<EcError> for LaunchError {
    fn from(e: EcError) -> Self {
        LaunchError::Ec(EcWrap(e))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LedMode {
    SolidColor = 0,
    PerKey,
    CycleAll,
    CycleLeftRight,
    CycleUpDown,
    CycleOutIn,
    CycleOutInDual,
    RainbowMovingChevron,
    CyclePinwheel,
    CycleSpiral,
    Raindrops,
    Splash,
    Multisplash,
    ActiveKeys,
    Disabled,
    Last,
}

impl TryFrom<u8> for LedMode {
    type Error = LaunchError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::SolidColor),
            1 => Ok(Self::PerKey),
            2 => Ok(Self::CycleAll),
            3 => Ok(Self::CycleLeftRight),
            4 => Ok(Self::CycleUpDown),
            5 => Ok(Self::CycleOutIn),
            6 => Ok(Self::CycleOutInDual),
            7 => Ok(Self::RainbowMovingChevron),
            8 => Ok(Self::CyclePinwheel),
            9 => Ok(Self::CycleSpiral),
            10 => Ok(Self::Raindrops),
            11 => Ok(Self::Splash),
            12 => Ok(Self::Multisplash),
            13 => Ok(Self::ActiveKeys),
            14 => Ok(Self::Disabled),
            15 => Ok(Self::Last),
            other => Err(Self::Error::UnknownLedMode(other)),
        }
    }
}

impl fmt::Display for LedMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SolidColor => write!(f, "Solid Color"),
            Self::PerKey => write!(f, "Per Key"),
            Self::CycleAll => write!(f, "Cycle All"),
            Self::CycleLeftRight => write!(f, "Cycle Left to Right"),
            Self::CycleUpDown => write!(f, "Cycle Up to Down"),
            Self::CycleOutIn => write!(f, "Cycle Out to In"),
            Self::CycleOutInDual => write!(f, "Cycle Out to In Dual"),
            Self::RainbowMovingChevron => write!(f, "Rainbow Chevron"),
            Self::CyclePinwheel => write!(f, "Pinwheel"),
            Self::CycleSpiral => write!(f, "Spiral"),
            Self::Raindrops => write!(f, "Raindrops"),
            Self::Splash => write!(f, "Splash"),
            Self::Multisplash => write!(f, "Multisplash"),
            Self::ActiveKeys => write!(f, "Active Keys"),
            Self::Disabled => write!(f, "Disabled"),
            Self::Last => write!(f, "Last"),
        }
    }
}

pub struct Launch {
    ec: Ec<Box<dyn Access>>,
    board: String,
    version: String,
    current_mode: LedMode,
}

impl Launch {
    pub fn try_new() -> Result<Self, LaunchError> {
        let api = HidApi::new()?;
        for info in api.device_list() {
            match (info.vendor_id(), info.product_id(), info.interface_number()) {
                (0x3384, 0x0001..=0x000A, 1) => {
                    let device = info.open_device(&api)?;
                    let access = AccessHid::new(device, 10, 100)?;

                    let (ec, board, version, current_mode) = unsafe {
                        let mut ec = Ec::new(access)?.into_dyn();

                        let data_size = ec.access().data_size();

                        let mut data = vec![0; data_size];

                        let board = {
                            let size = ec.board(&mut data)?;
                            data.truncate(size);
                            String::from_utf8(data.clone())?
                        };

                        let version = {
                            let size = ec.version(&mut data)?;
                            data.truncate(size);
                            String::from_utf8(data)?
                        };

                        let current_mode = {
                            let mode = ec.led_get_mode(0)?.0;
                            LedMode::try_from(mode)?
                        };

                        (ec, board, version, current_mode)
                    };

                    return Ok(Self {
                        ec,
                        board,
                        version,
                        current_mode,
                    });
                }
                _ => {}
            }
        }
        Err(LaunchError::DeviceNotFound)
    }

    pub fn board(&self) -> &String {
        &self.board
    }

    pub fn version(&self) -> &String {
        &self.version
    }

    pub fn current_mode(&self) -> LedMode {
        self.current_mode
    }

    pub fn set_led_mode(&mut self, mode: LedMode, speed: u8) -> Result<(), LaunchError> {
        let mode_raw = unsafe {
            self.ec.led_set_mode(0, mode as u8, speed)?;
            self.ec.led_get_mode(0)?.0
        };
        self.current_mode = LedMode::try_from(mode_raw)?;
        Ok(())
    }
}
