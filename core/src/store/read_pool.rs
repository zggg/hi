use std::path::Path;
use std::sync::{Condvar, Mutex};
use std::time::Duration;

use rusqlite::{Connection, OpenFlags};

use crate::error::{Error, Result};

use super::schema::verify_read_compatible;

struct PoolInner {
    idle: Vec<Connection>,
}

/// Small pool of read-only SQLite connections (WAL-safe concurrent readers).
///
/// Author: gz
pub struct ReadPool {
    inner: Mutex<PoolInner>,
    cvar: Condvar,
}

impl ReadPool {
    pub fn open(path: &Path, db_path: &str, pool_size: u32) -> Result<Self> {
        let size = pool_size.clamp(
            crate::config::MIN_READ_POOL_SIZE,
            crate::config::MAX_READ_POOL_SIZE,
        ) as usize;
        let mut idle = Vec::with_capacity(size);
        for _ in 0..size {
            let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
                .map_err(|e| Error::Message(format!("open read connection {}: {e}", path.display())))?;
            conn.busy_timeout(Duration::from_secs(5))
                .map_err(|e| Error::Message(format!("read busy_timeout: {e}")))?;
            verify_read_compatible(&conn, db_path)?;
            idle.push(conn);
        }
        Ok(Self {
            inner: Mutex::new(PoolInner { idle }),
            cvar: Condvar::new(),
        })
    }

    pub fn with_conn<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&Connection) -> Result<T>,
    {
        let conn = self.acquire()?;
        let result = f(&conn);
        self.release(conn);
        result
    }

    fn acquire(&self) -> Result<Connection> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|e| Error::Message(format!("read pool lock: {e}")))?;
        while guard.idle.is_empty() {
            guard = self
                .cvar
                .wait(guard)
                .map_err(|e| Error::Message(format!("read pool wait: {e}")))?;
        }
        guard.idle.pop().ok_or_else(|| {
            Error::Message("read pool empty after wait (internal error)".into())
        })
    }

    fn release(&self, conn: Connection) {
        if let Ok(mut guard) = self.inner.lock() {
            guard.idle.push(conn);
            self.cvar.notify_one();
        }
    }
}
