use egui_bench::markdown;
use egui_bench::update::fetch_text;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

const LIMIT: u64 = 2_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Readme,
    Changelog,
}

impl Kind {
    pub const ALL: [Kind; 2] = [Kind::Readme, Kind::Changelog];

    pub fn file(self) -> &'static str {
        match self {
            Kind::Readme => "README.md",
            Kind::Changelog => "CHANGELOG.md",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Kind::Readme => "readme",
            Kind::Changelog => "changelog",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Doc {
    pub repo: String,
    pub kind: Kind,
}

impl Doc {
    pub fn raw(&self) -> String {
        format!("https://raw.githubusercontent.com/{}/HEAD/{}", self.repo, self.kind.file())
    }

    pub fn page(&self) -> String {
        format!("https://github.com/{}/blob/HEAD/{}", self.repo, self.kind.file())
    }

    pub fn base(&self) -> String {
        format!("https://github.com/{}/blob/HEAD/", self.repo)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Loaded {
    Loading,
    Ready(Arc<String>),
    Failed(String),
}

type Held = HashMap<(String, Kind), Loaded>;

#[derive(Clone, Default)]
pub struct Library {
    held: Arc<Mutex<Held>>,
}

impl Library {
    fn lock(&self) -> MutexGuard<'_, Held> {
        self.held.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn get(&self, doc: &Doc, ctx: &egui::Context) -> Loaded {
        let key = (doc.repo.clone(), doc.kind);
        {
            let mut held = self.lock();
            if let Some(got) = held.get(&key) {
                return got.clone();
            }
            held.insert(key.clone(), Loaded::Loading);
        }
        let me = self.clone();
        let url = doc.raw();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let outcome = match fetch_text(&url, LIMIT) {
                Ok(text) => Loaded::Ready(Arc::new(text)),
                Err(e) => Loaded::Failed(e),
            };
            me.lock().insert(key, outcome);
            ctx.request_repaint();
        });
        Loaded::Loading
    }

    pub fn forget(&self, doc: &Doc) {
        self.lock().remove(&(doc.repo.clone(), doc.kind));
    }
}

#[derive(Default)]
pub struct Reader {
    pub open: Option<Doc>,
    pub library: Library,
    pub cache: markdown::Cache,
}

impl Reader {
    pub fn show(&mut self, repo: &str, kind: Kind) {
        self.open = Some(Doc { repo: repo.to_string(), kind });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_document_is_fetched_from_raw_and_opened_on_its_page() {
        let doc = Doc { repo: "v0l/gcad".into(), kind: Kind::Changelog };
        assert_eq!(doc.raw(), "https://raw.githubusercontent.com/v0l/gcad/HEAD/CHANGELOG.md");
        assert_eq!(doc.page(), "https://github.com/v0l/gcad/blob/HEAD/CHANGELOG.md");
        assert_eq!(doc.base(), "https://github.com/v0l/gcad/blob/HEAD/");
        assert_eq!(Kind::Readme.file(), "README.md");
    }

    #[test]
    fn a_failure_is_remembered_so_it_is_not_retried_every_frame() {
        let lib = Library::default();
        let doc = Doc { repo: "v0l/none".into(), kind: Kind::Readme };
        lib.lock().insert(("v0l/none".into(), Kind::Readme), Loaded::Failed("no".into()));
        assert_eq!(lib.get(&doc, &egui::Context::default()), Loaded::Failed("no".into()));
        lib.forget(&doc);
        assert_eq!(lib.get(&doc, &egui::Context::default()), Loaded::Loading);
    }
}
