use crate::error::WakeguardError;
use crate::service::worker;
use std::ffi::OsString;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use windows_service::define_windows_service;
use windows_service::service::{
    ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
    ServiceType,
};
use windows_service::service_control_handler::{
    self, ServiceControlHandlerResult, ServiceStatusHandle,
};
use windows_service::service_dispatcher;

const SERVICE_NAME: &str = "Wakeguard";
const ERROR_FAILED_SERVICE_CONTROLLER_CONNECT: i32 = 1063;

define_windows_service!(ffi_service_main, service_main);

pub fn run_or_console(interval: Duration, max_cycles: Option<usize>) -> Result<(), WakeguardError> {
    match service_dispatcher::start(SERVICE_NAME, ffi_service_main) {
        Ok(()) => Ok(()),
        Err(err) if is_not_service_context(&err) => {
            tracing::info!("service dispatcher unavailable, running in console mode");
            worker::run_periodic_loop(Some(interval), max_cycles)
        }
        Err(err) => Err(map_windows_service_error(err)),
    }
}

fn service_main(_arguments: Vec<OsString>) {
    if let Err(err) = run_service_runtime() {
        tracing::error!(error = %err, "service runtime failed");
    }
}

fn run_service_runtime() -> Result<(), WakeguardError> {
    let stop_requested = Arc::new(AtomicBool::new(false));
    let stop_requested_handler = stop_requested.clone();

    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            ServiceControl::Stop | ServiceControl::Shutdown => {
                stop_requested_handler.store(true, Ordering::SeqCst);
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };

    let status_handle = service_control_handler::register(SERVICE_NAME, event_handler)
        .map_err(map_windows_service_error)?;

    set_service_status(
        &status_handle,
        ServiceState::StartPending,
        ServiceControlAccept::empty(),
        ServiceExitCode::NO_ERROR,
    )?;

    set_service_status(
        &status_handle,
        ServiceState::Running,
        ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
        ServiceExitCode::NO_ERROR,
    )?;

    let interval = super::read_scan_interval();
    let max_cycles = super::read_max_cycles();
    let run_result = worker::run_periodic_loop_with_shutdown(Some(interval), max_cycles, || {
        stop_requested.load(Ordering::SeqCst)
    });

    set_service_status(
        &status_handle,
        ServiceState::StopPending,
        ServiceControlAccept::empty(),
        ServiceExitCode::NO_ERROR,
    )?;

    let exit_code = if run_result.is_ok() {
        ServiceExitCode::NO_ERROR
    } else {
        ServiceExitCode::ServiceSpecific(1)
    };

    set_service_status(
        &status_handle,
        ServiceState::Stopped,
        ServiceControlAccept::empty(),
        exit_code,
    )?;

    run_result
}

fn set_service_status(
    status_handle: &ServiceStatusHandle,
    current_state: ServiceState,
    controls_accepted: ServiceControlAccept,
    exit_code: ServiceExitCode,
) -> Result<(), WakeguardError> {
    let wait_hint = match current_state {
        ServiceState::StartPending | ServiceState::StopPending => Duration::from_secs(10),
        _ => Duration::default(),
    };

    let status = ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state,
        controls_accepted,
        exit_code,
        checkpoint: 0,
        wait_hint,
        process_id: None,
    };

    status_handle
        .set_service_status(status)
        .map_err(map_windows_service_error)
}

fn is_not_service_context(err: &windows_service::Error) -> bool {
    match err {
        windows_service::Error::Winapi(io_err) => {
            io_err.raw_os_error() == Some(ERROR_FAILED_SERVICE_CONTROLLER_CONNECT)
        }
        _ => false,
    }
}

fn map_windows_service_error(err: windows_service::Error) -> WakeguardError {
    WakeguardError::InvalidConfig(format!("windows service runtime error: {err}"))
}
