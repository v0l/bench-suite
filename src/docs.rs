use egui_bench::markdown;
use egui_bench::update::{fetch_bytes, fetch_text};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

const LIMIT: u64 = 2_000_000;
const PICTURE_LIMIT: u64 = 3_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Readme,
    Changelog,
}

impl Kind {
    pub fn file(self) -> &'static str {
        match self {
            Kind::Readme => "README.md",
            Kind::Changelog => "CHANGELOG.md",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Tab {
    Overview,
    Readme,
    Changelog,
}

impl Tab {
    pub fn label(self) -> &'static str {
        match self {
            Tab::Overview => "overview",
            Tab::Readme => "readme",
            Tab::Changelog => "changelog",
        }
    }

    pub fn kind(self) -> Option<Kind> {
        match self {
            Tab::Overview => None,
            Tab::Readme => Some(Kind::Readme),
            Tab::Changelog => Some(Kind::Changelog),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Open {
    pub repo: String,
    pub tab: Tab,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Picture {
    Looking,
    Bytes { uri: String, bytes: Arc<[u8]> },
    None,
    Failed(String),
}

fn picture_cache(repo: &str) -> std::path::PathBuf {
    crate::install::cache()
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(std::env::temp_dir)
        .join("pictures")
        .join(repo.replace('/', "-"))
}

pub fn read_cached_picture(dir: &std::path::Path) -> Option<(String, Vec<u8>)> {
    let uri = std::fs::read_to_string(dir.join("uri")).ok()?;
    if uri.trim().is_empty() {
        return Some((String::new(), Vec::new()));
    }
    let bytes = std::fs::read(dir.join("image")).ok()?;
    Some((uri.trim().to_string(), bytes))
}

pub fn write_cached_picture(dir: &std::path::Path, uri: &str, bytes: &[u8]) {
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    let _ = std::fs::write(dir.join("image"), bytes);
    let _ = std::fs::write(dir.join("uri"), uri);
}

#[derive(Clone, Default)]
pub struct Pictures {
    held: Arc<Mutex<HashMap<String, Picture>>>,
}

impl Pictures {
    fn lock(&self) -> MutexGuard<'_, HashMap<String, Picture>> {
        self.held.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn get(&self, repo: &str, ctx: &egui::Context) -> Picture {
        {
            let mut held = self.lock();
            if let Some(got) = held.get(repo) {
                return got.clone();
            }
            held.insert(repo.to_string(), Picture::Looking);
        }
        let me = self.clone();
        let repo = repo.to_string();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let outcome = find_picture(&repo);
            me.lock().insert(repo, outcome);
            ctx.request_repaint();
        });
        Picture::Looking
    }
}

fn find_picture(repo: &str) -> Picture {
    let dir = picture_cache(repo);
    if let Some((uri, bytes)) = read_cached_picture(&dir) {
        return match uri.is_empty() {
            true => Picture::None,
            false => Picture::Bytes { uri, bytes: bytes.into() },
        };
    }
    let readme = Doc { repo: repo.to_string(), kind: Kind::Readme };
    let text = match fetch_text(&readme.raw(), LIMIT) {
        Ok(t) => t,
        Err(e) => return Picture::Failed(e),
    };
    let Some(url) = first_screenshot(&text, repo) else {
        write_cached_picture(&dir, "", &[]);
        return Picture::None;
    };
    match fetch_bytes(&url, PICTURE_LIMIT) {
        Ok(bytes) => {
            write_cached_picture(&dir, &url, &bytes);
            Picture::Bytes { uri: url, bytes: bytes.into() }
        }
        Err(e) => Picture::Failed(e),
    }
}

#[derive(Default)]
pub struct Reader {
    pub open: Option<Open>,
    pub library: Library,
    pub pictures: Pictures,
    pub cache: markdown::Cache,
}

impl Reader {
    pub fn show(&mut self, repo: &str, tab: Tab) {
        self.open = Some(Open { repo: repo.to_string(), tab });
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
    fn a_picture_that_was_fetched_is_kept_on_disk_and_a_missing_one_is_remembered() {
        let dir = std::env::temp_dir().join(format!("bench-pic-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(read_cached_picture(&dir), None);
        write_cached_picture(&dir, "https://x/y.png", b"PNGDATA");
        assert_eq!(
            read_cached_picture(&dir),
            Some(("https://x/y.png".into(), b"PNGDATA".to_vec()))
        );
        write_cached_picture(&dir, "", &[]);
        assert_eq!(read_cached_picture(&dir).map(|(u, _)| u), Some(String::new()));
        let _ = std::fs::remove_dir_all(&dir);
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

pub fn first_screenshot(readme: &str, repo: &str) -> Option<String> {
    let base = format!("https://raw.githubusercontent.com/{repo}/HEAD/");
    let mut found: Vec<(usize, String)> = Vec::new();
    let mut at = 0;
    while let Some(i) = readme[at..].find("![") {
        let start = at + i;
        if let Some(open) = readme[start..].find("](") {
            let rest = &readme[start + open + 2..];
            let end = rest.find([')', ' ']).unwrap_or(rest.len());
            found.push((start, rest[..end].to_string()));
        }
        at = start + 2;
    }
    let mut at = 0;
    while let Some(i) = readme[at..].find("<img") {
        let start = at + i;
        let tag_end = readme[start..].find('>').map(|e| start + e).unwrap_or(readme.len());
        let tag = &readme[start..tag_end];
        if let Some(src) = tag.find("src=\"") {
            let rest = &tag[src + 5..];
            if let Some(q) = rest.find('"') {
                found.push((start, rest[..q].to_string()));
            }
        }
        at = start + 4;
    }
    found.sort_by_key(|(at, _)| *at);
    found
        .into_iter()
        .map(|(_, url)| url)
        .find(|url| {
            let path = url.split(['?', '#']).next().unwrap_or(url).to_ascii_lowercase();
            path.ends_with(".png") || path.ends_with(".jpg") || path.ends_with(".jpeg")
        })
        .map(|url| egui_bench::markdown::resolve(&url, &base))
}

#[cfg(test)]
mod screenshot_tests {
    use super::*;

    #[test]
    fn the_first_picture_in_a_readme_is_found_and_made_whole() {
        let readme = "<img src=\"assets/icon.svg\" width=\"64\">\n\n# gcad\n\n```\ncode\n```\n\n![The robot arm](docs/robot-arm.png)\n![b](docs/b.png)\n";
        assert_eq!(
            first_screenshot(readme, "v0l/gcad").as_deref(),
            Some("https://raw.githubusercontent.com/v0l/gcad/HEAD/docs/robot-arm.png")
        );
    }

    #[test]
    fn an_html_image_counts_and_a_logo_that_is_a_vector_does_not() {
        let readme =
            "<img src=\"logo.svg\">\n<p><img alt=\"x\" src=\"docs/shot.png\" width=\"560\"></p>\n";
        assert_eq!(
            first_screenshot(readme, "v0l/x").as_deref(),
            Some("https://raw.githubusercontent.com/v0l/x/HEAD/docs/shot.png")
        );
    }

    #[test]
    fn a_readme_with_no_picture_has_none() {
        assert_eq!(first_screenshot("# thermview\n\ntext only\n", "v0l/t"), None);
        assert_eq!(first_screenshot("![badge](https://x.y/b.svg)\n", "v0l/t"), None);
        assert_eq!(first_screenshot("", "v0l/t"), None);
    }

    #[test]
    fn a_picture_with_a_title_and_an_absolute_address_keeps_its_address() {
        let readme = "![shot](https://example.com/a/shot.PNG \"A title\")\n";
        assert_eq!(
            first_screenshot(readme, "v0l/t").as_deref(),
            Some("https://example.com/a/shot.PNG")
        );
    }
}
