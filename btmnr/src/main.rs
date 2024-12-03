use windows_service::{
    define_windows_service,
    service_dispatcher,
    service_control_handler::{self, ServiceControlHandler},
    service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode,
        ServiceState, ServiceStatus, ServiceType,
    },
};
use log::{info, error};
use std::{
    ffi::OsString,
    sync::{Arc, atomic::{AtomicBool, Ordering}},
    time::Duration
};
use tokio;

struct BluetoothManager {
    config_manager: ConfigManager,
    running: Arc<AtomicBool>,
}

impl BluetoothManager {
    fn new() -> Self {
        Self {
            config_manager: ConfigManager::new(),
            running: Arc::new(AtomicBool::new(true)),
        }
    }

    async fn monitor_audio_activity(&self) {
        let mut last_activity = std::time::Instant::now();
        let mut consecutive_errors = 0;
        
        while self.running.load(Ordering::Relaxed) {
            let config = self.config_manager.get_config();

            match self.check_and_handle_audio(&mut last_activity, &config).await {
                Ok(_) => {
                    consecutive_errors = 0;
                }
                Err(e) => {
                    error!("Error in audio monitoring: {:?}", e);
                    consecutive_errors += 1;
                    
                    if consecutive_errors >= 3 {
                        error!("Too many consecutive errors, waiting before retry");
                        tokio::time::sleep(Duration::from_secs(30)).await;
                        consecutive_errors = 0;
                    }
                }
            }

            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }

    async fn check_and_handle_audio(
        &self,
        last_activity: &mut std::time::Instant,
        config: &Config,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if AudioMonitor::is_audio_playing()? {
            *last_activity = std::time::Instant::now();
            if config.auto_connect {
                self.ensure_connected().await?;
            }
        } else if last_activity.elapsed() > Duration::from_secs(config.inactivity_timeout) {
            self.disconnect_device().await?;
        }
        Ok(())
    }
}

fn main() -> Result<(), windows_service::Error> {
    // Инициализация обработчика сервиса
    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            ServiceControl::Stop | ServiceControl::Shutdown => {
                info!("Service shutdown received");
                running.store(false, Ordering::Relaxed);
                ServiceControlHandlerResult::NoError
            }
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };

    // Регистрация обработчика
    let status_handle = service_control_handler::register(
        "BluetoothManager",
        event_handler
    )?;

    // Обновление статуса сервиса
    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;

    service_dispatcher::start("BluetoothManager", ffi_service_main)?;
    Ok(())
}

fn service_main(arguments: Vec<OsString>) {
    if let Err(e) = run_service(arguments) {
        error!("Service error: {}", e);
    }
}

fn run_service(_arguments: Vec<OsString>) -> Result<(), Box<dyn std::error::Error>> {
    simple_logging::log_to_file(
        "bluetooth_manager.log",
        log::LevelFilter::Info
    )?;

    let manager = BluetoothManager::new();
    
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async {
        manager.monitor_audio_activity().await;
    });

    Ok(())
}
