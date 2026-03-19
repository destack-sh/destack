use zbus::blocking::{Connection, Proxy};
use zbus::zvariant::OwnedObjectPath;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::diagnostic::PlatformErrorCode;

/// The systemd user-session bus name.
const SYSTEMD_BUS_NAME: &str = "org.freedesktop.systemd1";

/// The systemd manager object path.
const SYSTEMD_MANAGER_PATH: &str = "/org/freedesktop/systemd1";

/// The systemd manager interface name.
const SYSTEMD_MANAGER_INTERFACE: &str = "org.freedesktop.systemd1.Manager";

/// One blocking systemd manager connection on the user session bus.
pub(crate) struct SystemdManager {
    /// The user session bus connection.
    connection: Connection,
}

impl SystemdManager {
    /// Connect one blocking systemd manager on the user session bus.
    pub(crate) fn connect(operation: &'static str) -> RuntimeResult<Self> {
        let connection = Connection::session()
            .map_err(|error| systemd_error(operation, "user session bus", &error))?;
        let manager = Self { connection };

        manager.proxy(operation)?;

        Ok(manager)
    }

    /// Return one blocking manager proxy for this connection.
    fn proxy(&self, operation: &'static str) -> RuntimeResult<Proxy<'_>> {
        Proxy::new(
            &self.connection,
            SYSTEMD_BUS_NAME,
            SYSTEMD_MANAGER_PATH,
            SYSTEMD_MANAGER_INTERFACE,
        )
        .map_err(|error| systemd_error(operation, "manager proxy", &error))
    }

    /// Ask systemd to reload user units from disk.
    pub(crate) fn reload(&self, operation: &'static str) -> RuntimeResult<()> {
        let manager = self.proxy(operation)?;
        let _: () = manager
            .call("Reload", &())
            .map_err(|error| systemd_error(operation, "Manager::Reload", &error))?;

        Ok(())
    }

    /// Enable one persisted timer unit through the systemd manager.
    pub(crate) fn enable_unit(
        &self,
        unit_name: &str,
        operation: &'static str,
    ) -> RuntimeResult<()> {
        let manager = self.proxy(operation)?;
        let _: (bool, Vec<(String, String, String)>) = manager
            .call(
                "EnableUnitFiles",
                &(vec![unit_name.to_string()], false, true),
            )
            .map_err(|error| systemd_error(operation, "Manager::EnableUnitFiles", &error))?;

        Ok(())
    }

    /// Disable one persisted timer unit through the systemd manager.
    pub(crate) fn disable_unit_if_present(
        &self,
        unit_name: &str,
        operation: &'static str,
    ) -> RuntimeResult<()> {
        if !self.has_unit_file(unit_name, operation)? {
            return Ok(());
        }

        let manager = self.proxy(operation)?;
        let _: Vec<(String, String, String)> = manager
            .call("DisableUnitFiles", &(vec![unit_name.to_string()], false))
            .map_err(|error| systemd_error(operation, "Manager::DisableUnitFiles", &error))?;

        Ok(())
    }

    /// Start one user unit immediately through the systemd manager.
    pub(crate) fn start_unit(&self, unit_name: &str, operation: &'static str) -> RuntimeResult<()> {
        let manager = self.proxy(operation)?;
        let _: OwnedObjectPath = manager
            .call("StartUnit", &(unit_name, "replace"))
            .map_err(|error| systemd_error(operation, "Manager::StartUnit", &error))?;

        Ok(())
    }

    /// Stop one user unit immediately through the systemd manager.
    pub(crate) fn stop_unit_if_present(
        &self,
        unit_name: &str,
        operation: &'static str,
    ) -> RuntimeResult<()> {
        if !self.has_loaded_unit(unit_name, operation)? {
            return Ok(());
        }

        let manager = self.proxy(operation)?;
        let _: OwnedObjectPath = manager
            .call("StopUnit", &(unit_name, "replace"))
            .map_err(|error| systemd_error(operation, "Manager::StopUnit", &error))?;

        Ok(())
    }

    /// Return whether one unit is currently loaded into the manager.
    pub(crate) fn has_loaded_unit(
        &self,
        unit_name: &str,
        operation: &'static str,
    ) -> RuntimeResult<bool> {
        let manager = self.proxy(operation)?;
        let units: Vec<(
            String,
            String,
            String,
            String,
            String,
            String,
            OwnedObjectPath,
            u32,
            String,
            OwnedObjectPath,
        )> = manager
            .call("ListUnitsByNames", &(vec![unit_name.to_string()],))
            .map_err(|error| systemd_error(operation, "Manager::ListUnitsByNames", &error))?;

        Ok(!units.is_empty())
    }

    /// Return whether one unit file exists on disk for this manager.
    pub(crate) fn has_unit_file(
        &self,
        unit_name: &str,
        operation: &'static str,
    ) -> RuntimeResult<bool> {
        let manager = self.proxy(operation)?;
        let unit_files: Vec<(String, String)> = manager
            .call(
                "ListUnitFilesByPatterns",
                &(Vec::<String>::new(), vec![unit_name.to_string()]),
            )
            .map_err(|error| {
                systemd_error(operation, "Manager::ListUnitFilesByPatterns", &error)
            })?;

        Ok(!unit_files.is_empty())
    }
}

/// Map one systemd manager error into one runtime error.
fn systemd_error(
    operation: &'static str,
    detail: &str,
    error: &impl std::fmt::Display,
) -> Box<crate::diagnostic::RuntimeError> {
    RuntimeError::from(PlatformError::generic(
        Some(PlatformErrorCode::IoInvalidData),
        format!("{operation}: {detail} failed: {error}"),
    ))
    .boxed()
}
