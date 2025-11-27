//! Audio capture service layer
//!
//! This module provides utility functions and constants for audio capture management.
//! The actual stream handling is done in the commands module using async tasks.

use std::time::Duration;

/// Poll interval for checking the stop signal in capture tasks
#[allow(dead_code)]
pub const POLL_INTERVAL: Duration = Duration::from_millis(100);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_poll_interval_is_reasonable() {
        // Polling should be frequent enough to be responsive but not too aggressive
        assert!(POLL_INTERVAL >= Duration::from_millis(50));
        assert!(POLL_INTERVAL <= Duration::from_millis(500));
    }
}
