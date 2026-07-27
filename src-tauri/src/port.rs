use portpicker::pick_unused_port;
use crate::errors::{AppError, Result};

pub fn find_free_port() -> Result<u16> {
    pick_unused_port().ok_or(AppError::NoFreePort)
}

pub fn find_multiple_free_ports(count: usize) -> Result<Vec<u16>> {
    let mut ports = Vec::new();
    for _ in 0..count {
        if let Some(port) = pick_unused_port() {
            ports.push(port);
        } else {
            return Err(AppError::NoFreePort);
        }
    }
    Ok(ports)
}
