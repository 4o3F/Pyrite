use eframe::egui;

pub enum PresentAction {
    Stay,
}

pub fn ui(ui: &mut egui::Ui) -> PresentAction {
    ui.heading("Present");
    ui.add_space(8.0);
    ui.label("Presentation mode.");

    PresentAction::Stay
}
