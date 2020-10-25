use super::{
    as_handle::AsHandle, drop_handle, error::IntoResult, logging::log_diagnostics, Connection,
    Error,
};
use log::debug;
use odbc_sys::{
    AttrOdbcVersion, EnvironmentAttribute, HDbc, HEnv, Handle, HandleType, SQLAllocHandle,
    SQLSetEnvAttr, SqlReturn,
};
use std::ptr::null_mut;

/// An `Environment` is a global context, in which to access data.
///
/// Associated with an `Environment` is any information that is global in nature, such as:
///
/// * The `Environment`'s state
/// * The current environment-level diagnostics
/// * The handles of connections currently allocated on the environment
/// * The current stetting of each environment attribute
#[derive(Debug)]
pub struct Environment {
    /// Invariant: Should always point to a valid ODBC Environment
    handle: HEnv,
}

unsafe impl AsHandle for Environment {
    fn as_handle(&self) -> Handle {
        self.handle as Handle
    }

    fn handle_type(&self) -> HandleType {
        HandleType::Env
    }
}

impl Drop for Environment {
    fn drop(&mut self) {
        unsafe {
            drop_handle(self.handle as Handle, HandleType::Env);
        }
    }
}

impl Environment {
    /// An allocated ODBC Environment handle
    ///
    /// # Safety
    ///
    /// There may only be one Odbc environment in any process at any time. Take care using this
    /// function in unit tests, as these run in parallel by default in Rust. Also no library should
    /// probably wrap the creation of an odbc environment into a safe function call. This is because
    /// using two of these "safe" libraries at the same time in different parts of your program may
    /// lead to race condition thus violating Rust's safety guarantees.
    ///
    /// Creating one environment in your binary is safe however.
    pub unsafe fn new() -> Result<Self, Error> {
        let mut handle = null_mut();
        let (handle, info) = match SQLAllocHandle(HandleType::Env, null_mut(), &mut handle) {
            // We can't provide nay diagnostics, as we don't have
            SqlReturn::ERROR => return Err(Error::NoDiagnostics),
            SqlReturn::SUCCESS => (handle, false),
            SqlReturn::SUCCESS_WITH_INFO => (handle, true),
            other => panic!(
                "Unexpected Return value for allocating ODBC Environment: {:?}",
                other
            ),
        };

        debug!("ODBC Environment created.");

        let env = Environment {
            handle: handle as HEnv,
        };
        if info {
            log_diagnostics(&env);
        }
        Ok(env)
    }

    /// Declares which Version of the ODBC API we want to use. This is the first thing that should
    /// be done with any ODBC environment.
    pub fn declare_version(&self, version: AttrOdbcVersion) -> Result<(), Error> {
        unsafe {
            SQLSetEnvAttr(
                self.handle,
                EnvironmentAttribute::OdbcVersion,
                version.into(),
                0,
            )
            .into_result(self)
        }
    }

    /// Allocate a new connection handle. The `Connection` must not outlive the `Environment`.
    pub fn allocate_connection(&self) -> Result<Connection, Error> {
        let mut handle = null_mut();
        unsafe {
            SQLAllocHandle(HandleType::Dbc, self.as_handle(), &mut handle).into_result(self)?;
            Ok(Connection::new(handle as HDbc))
        }
    }

    /// Provides access to the raw ODBC environment handle.
    pub fn as_raw(&self) -> HEnv {
        self.handle
    }
}