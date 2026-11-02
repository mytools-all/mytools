//! Health-check command used by the frontend to verify the IPC bridge
//! is alive. Will be replaced by real commands as features come online.

#[tauri::command]
pub fn ping() -> &'static str {
    "pong"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ping_returns_pong() {
        assert_eq!(ping(), "pong");
    }
}
