#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct HealthcheckResponse {
  app: String,
  message: String,
  status: String,
}

#[tauri::command]
fn healthcheck() -> HealthcheckResponse {
  HealthcheckResponse {
    app: "Translat".to_owned(),
    message: "Translat desktop shell is connected and ready to grow.".to_owned(),
    status: "ok".to_owned(),
  }
}

fn main() {
  tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![healthcheck])
    .run(tauri::generate_context!())
    .expect("error while running the Translat desktop shell");
}

#[cfg(test)]
mod tests {
  use super::{healthcheck, HealthcheckResponse};

  #[test]
  fn healthcheck_returns_expected_shell_status() {
    assert_eq!(
      healthcheck(),
      HealthcheckResponse {
        app: "Translat".to_owned(),
        message: "Translat desktop shell is connected and ready to grow.".to_owned(),
        status: "ok".to_owned(),
      }
    );
  }
}
