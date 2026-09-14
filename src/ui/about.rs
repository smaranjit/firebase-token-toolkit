//! The footer strip and the About dialog it opens.

use eframe::egui;

/// Names credited in the About dialog.
///
/// Sourced from the `CONTRIBUTORS` file at the repository root so that adding
/// someone is a documentation change rather than a code edit. GitHub's
/// contributors graph remains the authoritative record and is always linked,
/// so a name missing from this file is a courtesy gap, not a lost credit.
fn contributors() -> Vec<&'static str> {
    parse_contributors(include_str!("../../CONTRIBUTORS"))
}

/// Split out from [`contributors`] so it can be tested against fixed input.
/// Asserting on the real file would mean the test fails the day someone is
/// actually credited, which is the opposite of useful.
fn parse_contributors(raw: &str) -> Vec<&str> {
    raw.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect()
}

/// Bottom strip: version, repository link, and the About toggle.
///
/// The version comes from Cargo.toml via `env!`, so the number a user quotes in
/// a bug report cannot drift from the binary they are running.
pub fn footer(ui: &mut egui::Ui, show_about: &mut bool) {
    ui.horizontal(|ui| {
        ui.weak(concat!("v", env!("CARGO_PKG_VERSION")));
        ui.separator();
        ui.hyperlink_to("GitHub", env!("CARGO_PKG_REPOSITORY"));
        ui.separator();
        if ui.small_button("About").clicked() {
            *show_about = !*show_about;
        }
    });
}

pub fn window(ctx: &egui::Context, open: &mut bool) {
    egui::Window::new("About")
        .open(open)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.heading("Firebase Token Toolkit");
            ui.weak(concat!("v", env!("CARGO_PKG_VERSION")));
            ui.add_space(6.0);
            ui.label(env!("CARGO_PKG_DESCRIPTION"));

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(6.0);

            ui.strong("Author");
            ui.label("Smaranjit Maiti");

            let names = contributors();
            if !names.is_empty() {
                ui.add_space(8.0);
                ui.strong("Contributors");
                for name in names {
                    ui.label(name);
                }
            }

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(6.0);

            ui.horizontal(|ui| {
                ui.weak("MIT licensed");
                ui.separator();
                ui.hyperlink_to("Source", env!("CARGO_PKG_REPOSITORY"));
                ui.hyperlink_to(
                    "Contributors",
                    concat!(env!("CARGO_PKG_REPOSITORY"), "/graphs/contributors"),
                );
                ui.hyperlink_to(
                    "Report an issue",
                    concat!(env!("CARGO_PKG_REPOSITORY"), "/issues"),
                );
            });
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comment_only_file_credits_nobody() {
        // Drives the "hide the section entirely" branch: an empty list must not
        // render a Contributors heading with nothing under it.
        let raw = "# a comment\n\n#   indented comment\n\n";
        assert!(parse_contributors(raw).is_empty());
    }

    #[test]
    fn names_are_read_and_trimmed() {
        let raw = "# header\n\nAda Lovelace\n  Grace Hopper  \n\n# trailing note\n";
        assert_eq!(
            parse_contributors(raw),
            vec!["Ada Lovelace", "Grace Hopper"]
        );
    }

    #[test]
    fn the_shipped_file_parses() {
        // Not asserting it is empty — that would break the day someone is
        // credited. Only that include_str! resolves and parsing does not panic.
        let _ = contributors();
    }
}
