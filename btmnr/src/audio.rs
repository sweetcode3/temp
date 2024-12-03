use windows::Win32::Media::Audio::{
    IAudioSessionManager2, IAudioSessionEnumerator,
    IAudioSessionControl2, IMMDevice, IMMDeviceEnumerator,
    MMDeviceEnumerator, eRender, eConsole,
};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};
use windows::core::ComInterface;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AudioError {
    #[error("Failed to create device enumerator")]
    EnumeratorError,
    #[error("Failed to get default audio endpoint")]
    EndpointError,
    #[error("Failed to get session manager")]
    SessionManagerError,
    #[error("Failed to get session enumerator")]
    SessionEnumError,
    #[error("Windows API error: {0}")]
    WindowsError(#[from] windows::core::Error),
}

pub struct AudioMonitor;

impl AudioMonitor {
    pub fn is_audio_playing() -> Result<bool, AudioError> {
        unsafe {
            let enumerator: IMMDeviceEnumerator = CoCreateInstance(
                &MMDeviceEnumerator,
                None,
                CLSCTX_ALL
            ).map_err(|e| AudioError::WindowsError(e))?;

            let device: IMMDevice = enumerator
                .GetDefaultAudioEndpoint(eRender, eConsole)
                .map_err(|_| AudioError::EndpointError)?;

            let session_manager: IAudioSessionManager2 = device
                .cast::<IAudioSessionManager2>()
                .map_err(|_| AudioError::SessionManagerError)?;

            let session_enum: IAudioSessionEnumerator = session_manager
                .GetSessionEnumerator()
                .map_err(|_| AudioError::SessionEnumError)?;

            let count = session_enum.GetCount()
                .map_err(|e| AudioError::WindowsError(e))?;

            for i in 0..count {
                if let Ok(session) = session_enum.GetSession(i) {
                    if let Ok(session2) = session.cast::<IAudioSessionControl2>() {
                        if let Ok(id) = session2.GetSessionInstanceIdentifier() {
                            if !id.is_null() {
                                return Ok(true);
                            }
                        }
                    }
                }
            }
            Ok(false)
        }
    }
}
