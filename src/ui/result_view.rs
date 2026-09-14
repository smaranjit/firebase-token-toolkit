use eframe::egui;
use serde_json::Value;

use crate::firebase::jwt::{decode_payload, ts_to_iso};

pub fn copy_to_clipboard(text: &str) -> Result<(), String> {
    arboard::Clipboard::new()
        .and_then(|mut cb| cb.set_text(text.to_string()))
        .map_err(|e| e.to_string())
}

pub fn token_block(ui: &mut egui::Ui, label: &str, token: &str) {
    ui.group(|ui| {
        // The outcome is stashed in egui's temp store rather than rendered
        // inside the `clicked()` branch: that branch is true for exactly one
        // frame, so the failure message vanished before anyone could read it.
        let status_id = ui.id().with(("copy-status", label));
        ui.horizontal(|ui| {
            ui.strong(label);
            if ui.button("Copy").clicked() {
                let status = match copy_to_clipboard(token) {
                    Ok(()) => String::new(),
                    Err(e) => format!("copy failed: {e}"),
                };
                ui.data_mut(|d| d.insert_temp(status_id, status));
            }
        });
        let status: String = ui.data(|d| d.get_temp(status_id).unwrap_or_default());
        if !status.is_empty() {
            ui.colored_label(egui::Color32::LIGHT_RED, status);
        }
        egui::ScrollArea::horizontal()
            .id_salt(format!("token-{label}"))
            .max_height(80.0)
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut token.to_string())
                        .desired_rows(3)
                        .desired_width(f32::INFINITY)
                        .font(egui::TextStyle::Monospace)
                        .interactive(false),
                );
            });
    });
}

pub fn jwt_claims(ui: &mut egui::Ui, token: &str) {
    let payload = match decode_payload(token) {
        Ok(p) => p,
        Err(_) => {
            ui.label("(could not decode token payload)");
            return;
        }
    };
    let obj = match payload {
        Value::Object(o) => o,
        _ => {
            ui.label("(payload was not a JSON object)");
            return;
        }
    };

    ui.collapsing("Decoded JWT claims", |ui| {
        egui_extras::TableBuilder::new(ui)
            .striped(true)
            .resizable(false)
            .column(egui_extras::Column::auto().at_least(120.0))
            .column(egui_extras::Column::remainder())
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.strong("Claim");
                });
                header.col(|ui| {
                    ui.strong("Value");
                });
            })
            .body(|mut body| {
                let mut keys: Vec<&String> = obj.keys().collect();
                keys.sort();
                for k in keys {
                    let v = &obj[k];
                    let value_text = match (k.as_str(), v) {
                        ("iat", Value::Number(n))
                        | ("exp", Value::Number(n))
                        | ("auth_time", Value::Number(n)) => n
                            .as_i64()
                            .map(|t| format!("{t} ({})", ts_to_iso(t)))
                            .unwrap_or_else(|| n.to_string()),
                        (_, Value::String(s)) => s.clone(),
                        (_, other) => other.to_string(),
                    };
                    body.row(18.0, |mut row| {
                        row.col(|ui| {
                            ui.monospace(k);
                        });
                        row.col(|ui| {
                            ui.add(
                                egui::Label::new(egui::RichText::new(value_text).monospace())
                                    .wrap()
                                    .selectable(true),
                            );
                        });
                    });
                }
            });
    });
}

pub fn error_block(ui: &mut egui::Ui, err: &str) {
    ui.colored_label(egui::Color32::LIGHT_RED, format!("Error: {err}"));
}
