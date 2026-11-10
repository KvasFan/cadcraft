use super::{DocState, Drawing, EngineError, Result, Session, Space, Value, Vec2, View, cmd};

/// Apply the user's dimension defaults to the current dimension style of a new drawing (values a
/// style refuses are skipped).
pub fn apply_dim_defaults(defaults: &serde_json::Map<String, serde_json::Value>, d: &mut Drawing) {
    if defaults.is_empty() {
        return;
    }
    let name = d.header.str("DIMSTYLE", "Standard");
    if let Some(style) = d.dim_styles.iter_mut().find(|s| s.name.eq_ignore_ascii_case(&name)) {
        let _ = style.apply_fields(defaults);
        d.sync_dim_vars();
    }
}

impl Session {
    /// Keep `defaults` for new drawings; open drawings keep their styles.
    pub fn set_dim_defaults(&mut self, defaults: serde_json::Map<String, serde_json::Value>) {
        self.dim_defaults = defaults;
    }

    /// The profile's defaults at start-up: kept for new drawings and, when the open drawing is
    /// the untouched first one (no file, nothing drawn or undoable), given to it too, since the
    /// startup drawing is made before the profile is read.
    pub fn restore_dim_defaults(&mut self, defaults: serde_json::Map<String, serde_json::Value>) {
        self.set_dim_defaults(defaults);
        if let [only] = self.docs.as_mut_slice()
            && only.path.is_none()
            && only.undo.is_empty()
            && only.redo.is_empty()
            && only.doc.model.is_empty()
        {
            let mut d = (*only.doc).clone();
            apply_dim_defaults(&self.dim_defaults, &mut d);
            only.doc = std::sync::Arc::new(d);
            only.saved = only.doc.clone();
        }
    }

    /// Append a new untitled drawing (`DrawingN`, metric or imperial default view), end a running command, make it active, bump the document epoch and return its index.
    pub fn new_drawing(&mut self, metric: bool) -> usize {
        self.untitled_counter += 1;
        let mut d = if metric { Drawing::new_metric() } else { Drawing::new_imperial() };
        apply_dim_defaults(&self.dim_defaults, &mut d);
        let mut st = DocState::new(d, &format!("Drawing{}", self.untitled_counter), None);
        st.set_view_quiet(if metric { View { center: Vec2::new(210.0, 148.5), height: 297.0, twist: 0.0 } } else { View::default() });
        self.end_running();
        self.docs.push(st);
        self.active = self.docs.len() - 1;
        self.bump_doc_epoch();
        self.active
    }

    /// Append a loaded drawing, fit the view to its model extents (aspect-corrected, minimum height 1e-6) when non-empty, end a running command, make it active, bump the epoch and return its index.
    pub fn open_drawing(&mut self, d: Drawing, title: &str, path: Option<String>) -> usize {
        let mut st = DocState::new(d, title, path);
        // The view the file was saved at is the one every other reader opens on. Fitting the
        // extents instead showed a drawing whose extents are dominated by far-off geometry at a
        // view hundreds of times its content, with the drawing sub-pixel and no way to zoom in
        // (#1602). The extents remain the fallback for a drawing that stores no view.
        let saved = crate::view::saved_view(&st.doc);
        let ext = st.doc.extents(&Space::Model);
        // The current tile (the first), or the lone untiled view, keeps its saved twist.
        let twist = cadcraft_geom::norm_angle(st.doc.vports.iter().find(|c| c.is_active()).and_then(|c| c.tiles.first()).map_or(0.0, |t| t.twist));
        if let Some(v) = saved {
            st.set_view_quiet(View { twist, ..v });
        } else if ext.is_empty() {
            st.set_view_quiet(View { twist, ..st.view() });
        } else {
            let (center, w, h) = View::twisted_box(ext, twist);
            st.set_view_quiet(View { center, height: (h.max(w * self.viewport_px.1 / self.viewport_px.0.max(1.0)) * 1.1).max(1e-6), twist });
        }
        self.end_running();
        self.docs.push(st);
        self.active = self.docs.len() - 1;
        self.bump_doc_epoch();
        self.active
    }

    /// Record that which drawing is open or active changed (screenshot binding).
    pub fn bump_doc_epoch(&mut self) {
        self.doc_epoch = self.doc_epoch.wrapping_add(1);
    }

    /// A command still waiting for input belongs to the drawing it started in: end it there
    /// before another drawing becomes current, so its undo step lands in that drawing.
    fn end_running(&mut self) {
        if self.running.is_some() {
            self.cancel();
        }
    }

    /// Add a file read off-thread with [`cmd::file::read_file`] as the active drawing (OPEN's fast
    /// half). A panic while adding it is reported as an error, never propagated.
    pub fn finish_open(&mut self, f: cmd::file::ReadFile) -> Result<Value> {
        let n = self.docs.len();
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| cmd::file::finish_open(self, f))) {
            Ok(r) => r,
            Err(_) => {
                self.docs.truncate(n);
                self.active = self.active.min(self.docs.len().saturating_sub(1));
                self.bump_doc_epoch();
                Err(EngineError::Internal("open".into(), "panic".into()))
            }
        }
    }
}
