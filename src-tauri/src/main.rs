#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod error;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![commands::healthcheck::healthcheck])
        .run(tauri::generate_context!())
        .expect("error while running the Translat desktop shell");
}

#[cfg(test)]
mod tests {
    use super::commands::healthcheck::healthcheck;

    #[test]
    fn healthcheck_returns_expected_shell_status() {
        let response = healthcheck().expect("healthcheck should succeed");

        assert_eq!(response.app_name, "Translat");
        assert_eq!(response.environment, "development");
        assert_eq!(response.status, "ok");
        assert!(!response.message.is_empty());
        assert!(!response.version.is_empty());
        assert!(response.checked_at > 0);
    }
}
