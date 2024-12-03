use windows::Win32::Devices::Bluetooth::{
    BluetoothFindFirstDevice,
    BluetoothFindDeviceClose,
    BLUETOOTH_DEVICE_INFO,
    BLUETOOTH_DEVICE_SEARCH_PARAMS,
    BluetoothAuthenticateDevice,
    BluetoothSetServiceState,
    BluetoothFindNextDevice,
};
use windows::Win32::Foundation::BOOL;
use windows::core::GUID;
use std::mem::zeroed;
use thiserror::Error;
use log::{error, info};

#[derive(Error, Debug)]
pub enum BluetoothError {
    #[error("Failed to find device")]
    DeviceNotFound,
    #[error("Failed to authenticate device")]
    AuthenticationError,
    #[error("Failed to set service state")]
    ServiceStateError,
    #[error("Failed to enumerate devices")]
    EnumerationError,
    #[error("Windows API error: {0}")]
    WindowsError(#[from] windows::core::Error),
}

const HANDSFREE_SERVICE_GUID: GUID = GUID::from_values(
    0x0000111E, 0x0000, 0x1000,
    [0x80, 0x00, 0x00, 0x80, 0x5F, 0x9B, 0x34, 0xFB]
);

pub struct BluetoothController {
    device_address: String,
}

impl BluetoothController {
    pub fn new(device_address: String) -> Self {
        Self { device_address }
    }

    pub async fn connect(&self) -> Result<(), BluetoothError> {
        unsafe {
            let (device_handle, device_info) = self.find_device(true)?;
            
            info!("Found target device, attempting to authenticate");
            BluetoothAuthenticateDevice(None, None, &device_info, None)
                .map_err(|_| {
                    error!("Authentication failed for device {}", self.device_address);
                    BluetoothError::AuthenticationError
                })?;

            info!("Setting up HandsFree service");
            BluetoothSetServiceState(
                None,
                &device_info,
                &HANDSFREE_SERVICE_GUID,
                1
            ).map_err(|_| {
                error!("Failed to enable HandsFree service");
                BluetoothError::ServiceStateError
            })?;

            BluetoothFindDeviceClose(device_handle);
            info!("Successfully connected to device {}", self.device_address);
            Ok(())
        }
    }

    pub async fn disconnect(&self) -> Result<(), BluetoothError> {
        unsafe {
            let (device_handle, device_info) = self.find_device(false)?;

            info!("Disabling HandsFree service");
            BluetoothSetServiceState(
                None,
                &device_info,
                &HANDSFREE_SERVICE_GUID,
                0
            ).map_err(|_| {
                error!("Failed to disable HandsFree service");
                BluetoothError::ServiceStateError
            })?;

            BluetoothFindDeviceClose(device_handle);
            info!("Successfully disconnected from device {}", self.device_address);
            Ok(())
        }
    }

    unsafe fn find_device(&self, include_inquiry: bool) -> Result<(isize, BLUETOOTH_DEVICE_INFO), BluetoothError> {
        let mut params: BLUETOOTH_DEVICE_SEARCH_PARAMS = zeroed();
        params.dwSize = std::mem::size_of::<BLUETOOTH_DEVICE_SEARCH_PARAMS>() as u32;
        params.fReturnAuthenticated = BOOL::from(true);
        params.fReturnConnected = BOOL::from(true);
        params.fReturnRemembered = BOOL::from(true);
        params.fIssueInquiry = BOOL::from(include_inquiry);
        params.cTimeoutMultiplier = 1;

        let mut device_info: BLUETOOTH_DEVICE_INFO = zeroed();
        device_info.dwSize = std::mem::size_of::<BLUETOOTH_DEVICE_INFO>() as u32;

        let device_handle = BluetoothFindFirstDevice(&params, &mut device_info)
            .map_err(|e| {
                error!("Failed to start device enumeration: {:?}", e);
                BluetoothError::EnumerationError
            })?;

        let mut found = self.is_target_device(&device_info);
        
        while !found {
            match BluetoothFindNextDevice(device_handle, &mut device_info) {
                Ok(_) => {
                    found = self.is_target_device(&device_info);
                }
                Err(_) => {
                    BluetoothFindDeviceClose(device_handle);
                    return Err(BluetoothError::DeviceNotFound);
                }
            }
        }

        if found {
            Ok((device_handle, device_info))
        } else {
            BluetoothFindDeviceClose(device_handle);
            Err(BluetoothError::DeviceNotFound)
        }
    }

    fn is_target_device(&self, device_info: &BLUETOOTH_DEVICE_INFO) -> bool {
        unsafe {
            let address = format!("{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
                device_info.Address.Anonymous.rgBytes[5],
                device_info.Address.Anonymous.rgBytes[4],
                device_info.Address.Anonymous.rgBytes[3],
                device_info.Address.Anonymous.rgBytes[2],
                device_info.Address.Anonymous.rgBytes[1],
                device_info.Address.Anonymous.rgBytes[0],
            );
            address == self.device_address
        }
    }
}
