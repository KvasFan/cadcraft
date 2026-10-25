//! The Save As dialog: the file type, the DXF/DWG version and binary DXF, chosen before the
//! host's file picker, which cannot report the type the user picked. "Save..." opens the picker
//! with the chosen type and runs `saveas`.
//!
//! Two file types are for the Italian cadastre's DOCFA: "DXF compatible with DOCFA" and "DWG
//! compatible with DOCFA". They write the DOCFA profile (`docs/docfa.md`) in the AutoCAD version
//! Docfa 4 reads, so the dialog asks for no version, only the drawing's unit (the file is in
//! metres). They are exports: the drawing keeps its own file.
//!
//! QSAVE of a drawing that has no file yet skips this dialog and goes straight to the picker
//! (`ui.saveas.pick`): the close prompt's "Save" relies on that save finishing in the same call.

use egui::{RichText, vec2};
use serde_json::{Value, json};

use crate::theme::Tokens;
use crate::{CadApp, SaveFormat, SaveVersion};

/// The version the DOCFA file types write: AutoCAD 2000, what Docfa 4 reads in DXF and DWG.
pub const DOCFA_VERSION: SaveVersion = SaveVersion::R2000;

/// A file type the dialog saves as.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileType {
    Dxf,
    Dwg,
    /// DXF with the DOCFA profile.
    DocfaDxf,
    /// DWG with the DOCFA profile.
    DocfaDwg,
    Dwt,
    Svg,
    Png,
    Pdf,
}

impl FileType {
    pub const ALL: [FileType; 8] =
        [FileType::Dxf, FileType::Dwg, FileType::DocfaDxf, FileType::DocfaDwg, FileType::Dwt, FileType::Svg, FileType::Png, FileType::Pdf];

    pub fn ext(self) -> &'static str {
        match self {
            FileType::Dxf | FileType::DocfaDxf => "dxf",
            FileType::Dwg | FileType::DocfaDwg => "dwg",
            FileType::Dwt => "dwt",
            FileType::Svg => "svg",
            FileType::Png => "png",
            FileType::Pdf => "pdf",
        }
    }

    /// The label of the type (the file picker's filter names, translated there too).
    fn label(self) -> &'static str {
        match self {
            FileType::Dxf => crate::tl!("Drawing (DXF)"),
            FileType::Dwg => crate::tl!("Drawing (DWG)"),
            FileType::DocfaDxf => crate::tl!("DXF compatible with DOCFA"),
            FileType::DocfaDwg => crate::tl!("DWG compatible with DOCFA"),
            FileType::Dwt => crate::tl!("Drawing Template (DWT)"),
            FileType::Svg => crate::tl!("SVG"),
            FileType::Png => crate::tl!("PNG"),
            FileType::Pdf => crate::tl!("PDF"),
        }
    }

    /// The plain type of an extension (a DOCFA type is only ever chosen).
    pub fn from_ext(ext: &str) -> Option<FileType> {
        let ext = ext.trim().trim_start_matches('.');
        FileType::ALL.into_iter().filter(|t| !t.is_docfa()).find(|t| t.ext().eq_ignore_ascii_case(ext))
    }

    /// A drawing file whose version the dialog asks for (DXF, DWG, DWT); the DOCFA types have
    /// theirs, images none.
    fn versioned(self) -> bool {
        matches!(self, FileType::Dxf | FileType::Dwg | FileType::Dwt)
    }

    /// One of the two DOCFA types.
    pub fn is_docfa(self) -> bool {
        matches!(self, FileType::DocfaDxf | FileType::DocfaDwg)
    }
}

/// The drawing's unit, for the DOCFA types (the file is in metres).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    M,
    Cm,
    Mm,
}

impl Unit {
    pub const ALL: [Unit; 3] = [Unit::M, Unit::Cm, Unit::Mm];

    pub fn as_str(self) -> &'static str {
        match self {
            Unit::M => "m",
            Unit::Cm => "cm",
            Unit::Mm => "mm",
        }
    }

    /// The unit the drawing's `$INSUNITS` names (4 mm, 5 cm, 6 m), else metres.
    fn from_insunits(v: i64) -> Unit {
        match v {
            4 => Unit::Mm,
            5 => Unit::Cm,
            _ => Unit::M,
        }
    }
}

/// The dialog's choices until Save.
#[derive(Clone, Debug, PartialEq)]
pub struct SaveAsState {
    pub file_type: FileType,
    pub version: SaveVersion,
    pub binary: bool,
    pub unit: Unit,
}

impl SaveAsState {
    /// The choices a drawing starts from: its own file type and version (an untitled drawing:
    /// the default save format and version), its binary flag, its unit.
    pub fn for_app(app: &CadApp) -> SaveAsState {
        let st = app.session.state().ok();
        let untitled = st.as_ref().is_none_or(|s| s.path.is_none());
        let title = st.as_ref().map(|s| s.title.clone()).unwrap_or_default();
        let from_title = std::path::Path::new(&title).extension().and_then(|e| FileType::from_ext(&e.to_string_lossy()));
        let default_type = match app.ui.save_format {
            SaveFormat::Dxf => FileType::Dxf,
            SaveFormat::Dwg => FileType::Dwg,
        };
        let file_type = from_title.filter(|t| t.versioned()).unwrap_or(default_type);
        let doc = app.session.doc().ok();
        let own = doc.and_then(|d| d.header.get("ACADVER").and_then(cadcraft_doc::HVal::as_str).and_then(SaveVersion::for_acadver));
        let version = match (untitled, own) {
            (false, Some(v)) => v,
            _ => SaveVersion::parse(&app.ui.save_version).unwrap_or_default(),
        };
        let unit = doc.map_or(Unit::M, |d| Unit::from_insunits(d.header.i64("INSUNITS", 0)));
        SaveAsState { file_type, version, binary: st.as_ref().is_some_and(|s| s.binary_dxf) && file_type == FileType::Dxf, unit }
    }

    /// The `saveas` parameters for `path`.
    pub fn params(&self, path: &str) -> Value {
        let mut p = json!({ "path": path });
        if let Some(o) = p.as_object_mut() {
            if self.file_type.is_docfa() {
                o.insert("docfa".into(), json!(true));
                o.insert("version".into(), json!(DOCFA_VERSION.year()));
                o.insert("unit".into(), json!(self.unit.as_str()));
            } else if self.file_type.versioned() {
                o.insert("version".into(), json!(self.version.year()));
                if self.binary && self.file_type == FileType::Dxf && self.version.lossless() {
                    o.insert("binary".into(), json!(true));
                }
            }
        }
        p
    }

    /// The versions the file type can be saved in: every one the format has (R12 for DXF only).
    pub fn versions(&self) -> Vec<SaveVersion> {
        SaveVersion::ALL.into_iter().filter(|v| *v != SaveVersion::R12 || self.file_type == FileType::Dxf).collect()
    }

    /// The file name the picker suggests: the drawing's name with the chosen extension (a DOCFA
    /// export is named apart, beside the drawing).
    pub fn suggested_name(&self, title: &str) -> String {
        let stem = std::path::Path::new(title).file_stem().map(|s| s.to_string_lossy().to_string()).filter(|s| !s.trim().is_empty());
        let stem = stem.unwrap_or_else(|| "Drawing".into());
        if self.file_type.is_docfa() { format!("{stem}-docfa.{}", self.file_type.ext()) } else { format!("{stem}.{}", self.file_type.ext()) }
    }

    /// Choices that don't go together are settled: DWG has no R12, binary DXF is 2000 or later.
    fn settle(&mut self) {
        if self.file_type != FileType::Dxf {
            self.binary = false;
        }
        if self.file_type != FileType::Dxf && self.version == SaveVersion::R12 {
            self.version = SaveVersion::R2000;
        }
        if !self.version.lossless() {
            self.binary = false;
        }
    }
}

/// Open the dialog for the active drawing.
pub fn open(app: &mut CadApp) {
    app.saveas = Some(SaveAsState::for_app(app));
    app.ui.dialog = Some("saveas".into());
}

/// Open the dialog on a DOCFA type (File > Save as … compatible with DOCFA, and the Options'
/// Open and Save tab).
pub fn open_docfa(app: &mut CadApp, file_type: FileType) {
    let mut st = SaveAsState::for_app(app);
    st.file_type = file_type;
    app.saveas = Some(st);
    app.ui.dialog = Some("saveas".into());
}

/// `path` with the chosen extension when the picker gave none (or a bare name).
fn with_extension(path: &str, ext: &str) -> String {
    let has = std::path::Path::new(path).extension().is_some_and(|e| !e.is_empty());
    if has { path.to_string() } else { format!("{path}.{ext}") }
}

/// Ask the host where to save and run `saveas` with the dialog's choices. `Some(true)` once
/// saved, `Some(false)` when the picker was cancelled or the save failed (the dialog stays),
/// `None` without a picker (reported on the command line).
pub fn save(app: &mut CadApp) -> Option<bool> {
    let st = app.saveas.clone().unwrap_or_else(|| SaveAsState::for_app(app));
    let title = app.session.state().map(|s| s.title.clone()).unwrap_or_default();
    let Some(pick) = app.services.pick_save.as_ref() else {
        app.session
            .echo(r#"SAVEAS: there is no file dialog here; run it with parameters, e.g. saveas {"path": "plan.dxf", "docfa": true}"#.to_string());
        return None;
    };
    let Some(path) = pick(&st.suggested_name(&title)) else { return Some(false) };
    let path = with_extension(&path, st.file_type.ext());
    match app.run("saveas", st.params(&path)) {
        Ok(r) => {
            if let Some(p) = r.get("path").and_then(Value::as_str)
                && st.file_type.versioned()
            {
                app.session.echo(format!("Saved {p}"));
            }
            Some(true)
        }
        Err(_) => Some(false),
    }
}

/// A dimmed, wrapping note.
fn note(ui: &mut egui::Ui, text: &str) {
    ui.add(egui::Label::new(RichText::new(text).color(Tokens::get().text_dim)).wrap());
}

pub fn dialog(app: &mut CadApp, ctx: &egui::Context, open: &mut bool) {
    if app.saveas.is_none() {
        app.saveas = Some(SaveAsState::for_app(app));
    }
    let Some(mut st) = app.saveas.clone() else { return };
    let (mut save, mut cancel) = (false, false);
    let center = ctx.content_rect().center();
    egui::Window::new(crate::tl!("Save As"))
        .id(egui::Id::unique("Save As"))
        .open(open)
        .collapsible(false)
        .resizable(false)
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(center)
        .show(ctx, |ui| {
            egui::Grid::new("saveas_grid").num_columns(2).spacing(vec2(12.0, 6.0)).show(ui, |ui| {
                ui.label(crate::tl!("File type"));
                egui::ComboBox::from_id_salt("saveas_type").selected_text(st.file_type.label()).width(260.0).show_ui(ui, |ui| {
                    for ft in FileType::ALL {
                        let r = ui.selectable_value(&mut st.file_type, ft, ft.label());
                        if ft.is_docfa() {
                            r.on_hover_text(crate::tl!(
                                "DXF or DWG as DOCFA imports it: model space only, block references exploded, no hatches, colours only on the DOCFA_POLIGONI polygons, ASCII text, in metres. The drawing keeps its own file."
                            ));
                        }
                    }
                });
                ui.end_row();
                if st.file_type.versioned() {
                    ui.label(crate::tl!("Version"));
                    let versions = st.versions();
                    egui::ComboBox::from_id_salt("saveas_version").selected_text(version_label(st.version)).width(260.0).show_ui(ui, |ui| {
                        for v in versions {
                            ui.selectable_value(&mut st.version, v, version_label(v));
                        }
                    });
                    ui.end_row();
                }
                if st.file_type == FileType::Dxf {
                    ui.label("");
                    ui.add_enabled_ui(st.version.lossless(), |ui| ui.checkbox(&mut st.binary, crate::tl!("Binary DXF")));
                    ui.end_row();
                }
                if st.file_type.is_docfa() {
                    ui.label(crate::tl!("Drawing unit"));
                    ui.horizontal(|ui| {
                        for u in Unit::ALL {
                            ui.radio_value(&mut st.unit, u, u.as_str());
                        }
                    });
                    ui.end_row();
                }
            });
            st.settle();
            if st.file_type.is_docfa() {
                note(
                    ui,
                    crate::tl!(
                        "An export for DOCFA: a reduced copy of the drawing, which keeps its own file. Closed polylines on layer DOCFA_POLIGONI are the surface polygons; their colour (1–8) is the area class."
                    ),
                );
                if st.file_type == FileType::DocfaDwg {
                    note(ui, crate::tl!("DWG has no polygon layer for DOCFA: it computes the surfaces inside DOCFA."));
                }
            } else if st.file_type.versioned() && st.version == SaveVersion::R12 {
                note(
                    ui,
                    crate::tl!(
                        "R12 is a reduced copy of the drawing: curves, hatches, multiline text, dimensions and layouts become lines, polylines and text."
                    ),
                );
            }
            ui.separator();
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    save = ui.button(crate::tl!("Save...")).clicked();
                    cancel = ui.button(crate::tl!("Cancel")).clicked();
                });
            });
        });
    app.saveas = Some(st);
    if cancel {
        *open = false;
    }
    if save && save_from_dialog(app) {
        *open = false;
    }
}

/// Save from the dialog: closed once saved, or when there is no picker to ask.
fn save_from_dialog(app: &mut CadApp) -> bool {
    save(app) != Some(false)
}

/// A version's label: its year, and R12 by its release name.
pub fn version_label(v: SaveVersion) -> String {
    match v {
        SaveVersion::R12 => format!("R12 ({})", crate::tl!("AutoCAD Release 12")),
        other => other.year().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use cadcraft_engine::Session;
    use serde_json::json;

    use super::*;
    use crate::Services;

    fn app_with_picker(answer: Option<String>) -> (CadApp, Rc<RefCell<Vec<String>>>) {
        let asked = Rc::new(RefCell::new(Vec::new()));
        let log = Rc::clone(&asked);
        let services = Services {
            pick_open: None,
            pick_save: Some(Box::new(move |name: &str| {
                log.borrow_mut().push(name.to_string());
                answer.clone()
            })),
            ..Services::default()
        };
        (CadApp::new(Session::new(), services), asked)
    }

    #[test]
    fn save_as_opens_the_dialog_with_the_drawings_own_type_and_version() {
        let (mut app, asked) = app_with_picker(Some("x.dxf".into()));
        app.start("ui.saveas");
        assert_eq!(app.ui.dialog.as_deref(), Some("saveas"));
        assert!(asked.borrow().is_empty(), "the picker waits for Save...");
        let st = app.saveas.clone().unwrap();
        assert_eq!((st.file_type, st.version, st.binary), (FileType::Dxf, SaveVersion::R2018, false));
        // A drawing opened as a 2000 DWG in millimetres.
        let mut d = cadcraft_doc::Drawing::new_metric();
        d.header.set_str("ACADVER", "AC1015");
        d.header.set_i64("INSUNITS", 4);
        app.session.open_drawing(d, "plan.dwg", Some("/tmp/plan.dwg".into()));
        open(&mut app);
        let st = app.saveas.clone().unwrap();
        assert_eq!((st.file_type, st.version, st.unit), (FileType::Dwg, SaveVersion::R2000, Unit::Mm));
        assert_eq!(st.suggested_name("plan.dwg"), "plan.dwg");
        assert_eq!(st.params("/tmp/plan.dwg"), json!({ "path": "/tmp/plan.dwg", "version": "2000" }));
    }

    #[test]
    fn the_docfa_types_need_no_version_and_carry_none_in_their_names() {
        for ft in [FileType::DocfaDxf, FileType::DocfaDwg] {
            assert!(ft.is_docfa() && !ft.versioned());
            assert!(!ft.label().contains("2000") && ft.label().contains("DOCFA"), "{}", ft.label());
        }
        assert_eq!(FileType::from_ext("dxf"), Some(FileType::Dxf), "an extension is never a DOCFA type");
        assert_eq!(FileType::from_ext(".DWG"), Some(FileType::Dwg));
        // DXF: the profile at the version Docfa 4 reads, with the unit; the export is named apart.
        let st = SaveAsState { file_type: FileType::DocfaDxf, version: SaveVersion::R2018, binary: true, unit: Unit::Mm };
        assert_eq!(st.params("/tmp/plan-docfa.dxf"), json!({ "path": "/tmp/plan-docfa.dxf", "docfa": true, "version": "2000", "unit": "mm" }));
        assert_eq!(st.suggested_name("Plan.dwg"), "Plan-docfa.dxf");
        let st = SaveAsState { file_type: FileType::DocfaDwg, unit: Unit::Cm, ..st };
        assert_eq!(st.params("/tmp/plan-docfa.dwg"), json!({ "path": "/tmp/plan-docfa.dwg", "docfa": true, "version": "2000", "unit": "cm" }));
        assert_eq!(st.suggested_name("Plan"), "Plan-docfa.dwg");
        // No version is labelled for DOCFA any more: the file types are.
        for v in SaveVersion::ALL {
            assert!(!version_label(v).contains("DOCFA"), "{}", version_label(v));
        }
    }

    #[test]
    fn choices_settle_and_become_saveas_parameters() {
        // R12 plain, then DWG: DWG has no R12.
        let mut st = SaveAsState { file_type: FileType::Dxf, version: SaveVersion::R12, binary: true, unit: Unit::M };
        st.settle();
        assert!(!st.binary, "R12 is ASCII");
        assert_eq!(st.params("a.dxf"), json!({ "path": "a.dxf", "version": "R12" }));
        st.file_type = FileType::Dwg;
        st.settle();
        assert_eq!(st.version, SaveVersion::R2000);
        assert_eq!(st.versions().first(), Some(&SaveVersion::R2000), "no R12 for DWG");
        // Images carry no version; binary DXF only at 2000 and later.
        let st = SaveAsState { file_type: FileType::Pdf, version: SaveVersion::R2018, binary: true, unit: Unit::M };
        assert_eq!(st.params("a.pdf"), json!({ "path": "a.pdf" }));
        let st = SaveAsState { file_type: FileType::Dxf, version: SaveVersion::R2004, binary: true, unit: Unit::M };
        assert_eq!(st.params("a.dxf"), json!({ "path": "a.dxf", "version": "2004", "binary": true }));
        assert_eq!(with_extension("/tmp/plan", "dxf"), "/tmp/plan.dxf");
        assert_eq!(with_extension("/tmp/plan.DXF", "dxf"), "/tmp/plan.DXF");
    }

    #[test]
    fn save_asks_the_picker_with_the_chosen_type_and_keeps_the_dialog_when_cancelled() {
        let (mut app, asked) = app_with_picker(None);
        open(&mut app);
        if let Some(st) = app.saveas.as_mut() {
            st.file_type = FileType::Svg;
        }
        assert_eq!(save(&mut app), Some(false));
        assert_eq!(*asked.borrow(), vec!["Drawing1.svg".to_string()]);
        // The DOCFA commands open the dialog on their type; the export is named apart.
        app.start("ui.saveas.docfa.dxf");
        assert_eq!((app.ui.dialog.as_deref(), app.saveas.as_ref().map(|s| s.file_type)), (Some("saveas"), Some(FileType::DocfaDxf)));
        assert_eq!(save(&mut app), Some(false));
        assert_eq!(asked.borrow().last().map(String::as_str), Some("Drawing1-docfa.dxf"));
        app.start("ui.saveas.docfa.dwg");
        assert_eq!(app.saveas.as_ref().map(|s| s.file_type), Some(FileType::DocfaDwg));
        // Without a picker the command line says how to save.
        let mut app = CadApp::new(Session::new(), Services::default());
        assert_eq!(save(&mut app), None);
        assert!(app.session.log.iter().any(|l| l.contains("no file dialog")));
    }

    #[test]
    fn qsave_of_an_untitled_drawing_goes_straight_to_the_picker() {
        let (mut app, asked) = app_with_picker(None);
        app.start("qsave");
        assert_eq!(*asked.borrow(), vec!["Drawing1.dxf".to_string()]);
        assert!(app.ui.dialog.is_none());
    }

    #[test]
    fn the_dialog_renders_its_choices() {
        let (mut app, _) = app_with_picker(None);
        open(&mut app);
        let ctx = egui::Context::default();
        let input =
            egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200.0, 800.0))), ..Default::default() };
        for _ in 0..2 {
            let mut output = ctx.run_ui(input.clone(), |ui| app.ui(ui));
            output.textures_delta.clear();
        }
        assert_eq!(app.ui.dialog.as_deref(), Some("saveas"));
        assert!(app.saveas.is_some());
    }
}
