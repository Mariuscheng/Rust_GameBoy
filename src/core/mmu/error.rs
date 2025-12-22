use thiserror::Error;

/// MMU 存取錯誤
#[derive(Error, Debug)]
pub enum MMUError {
    #[error("無效的記憶體地址: {0:#06X}")]
    InvalidAddress(u16),

    #[error("VRAM 不可訪問: {0}")]
    VRAMInaccessible(String),

    #[error("OAM 不可訪問: {0}")]
    OAMInaccessible(String),

    #[error("寫入只讀記憶體: 地址 {0:#06X}")]
    WriteToROM(u16),

    #[error("讀取無效區域: 地址 {0:#06X}")]
    ReadFromInvalid(u16),

    #[error("MBC 控制器錯誤: {0}")]
    MBCError(String),

    #[error("記憶體存取錯誤: {0}")]
    AccessError(String),
}

impl From<MMUError> for crate::error::Error {
    fn from(error: MMUError) -> Self {
        crate::error::Error::Hardware(crate::error::HardwareError::MMU(error.to_string()))
    }
}
