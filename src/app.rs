use crate::catalog::{SUITE, TOOLS, Target, Tool, for_path, locate};
use crate::skills::{self, Placed};
use crate::{install, launcher, source};
use egui_bench::prelude::Browser;
use egui_bench::update::{self, Install, Release, State, Updater};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Instant;

const RECENT_MAX: usize = 6;
const AGENT_OPEN_KEY: &str = "agent.open";
const KEEP_SOURCE_KEY: &str = "source.keep";
const TOOLS_ENV: &str = "BENCH_SUITE_TOOLS";

pub struct Slot {
    pub tool: &'static Tool,
    pub exe: Option<PathBuf>,
    pub managed: bool,
    pub updater: Updater,
    pub settled: bool,
    pub running: Vec<Child>,
    pub recent: Vec<PathBuf>,
    pub skills: Result<Vec<Placed>, String>,
}

impl Slot {
    pub fn offer(&self) -> Option<Release> {
        match self.updater.state() {
            State::Newer(r) => Some(r),
            _ => None,
        }
    }

    pub fn behind(&self) -> bool {
        self.exe.is_some() && self.updater.current().is_some() && self.offer().is_some()
    }

    pub fn wants_install(&self, target: Option<Target>) -> bool {
        (self.exe.is_none() || self.behind())
            && !self.updater.install_state().busy()
            && self.installable(target).is_some()
    }

    pub fn installable(&self, target: Option<Target>) -> Option<(Release, update::Asset)> {
        let (release, _, asset) = self.choice(target)?;
        Some((release, asset))
    }

    /// The release's best build for this machine: the platform's own installer when it has one,
    /// otherwise a plain archive.
    pub fn choice(
        &self,
        target: Option<Target>,
    ) -> Option<(Release, crate::catalog::Package, update::Asset)> {
        let release = self.offer()?;
        let target = target?;
        let (package, asset) = self
            .tool
            .packages(target, &release.tag)
            .into_iter()
            .find_map(|p| release.asset(&p.name).cloned().map(|a| (p, a)))?;
        Some((release, package, asset))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Filter {
    All,
    Installed,
    Updates,
    Missing,
}

impl Filter {
    pub fn admits(self, slot: &Slot) -> bool {
        match self {
            Filter::All => true,
            Filter::Installed => slot.exe.is_some(),
            Filter::Updates => slot.behind(),
            Filter::Missing => slot.exe.is_none(),
        }
    }
}

pub enum Act {
    Launch(usize, Option<PathBuf>),
    Pick(usize),
    Forget(usize),
    Install(usize),
    InstallAll,
    Uninstall(usize),
    Remove(usize, bool),
    SourceOpen(usize),
    SourceFetch(usize),
    SourceVendor(usize),
    Settings,
    Reveal(usize),
    RevealPath(PathBuf),
    Check,
    UpdateSelf,
    Restart,
    Url(String),
    ShowAll,
    Page(usize, crate::docs::Tab),
    SuitePage(crate::docs::Tab),
}

pub struct Hub {
    pub slots: Vec<Slot>,
    pub suite: Updater,
    pub target: Option<Target>,
    pub picking: Option<(usize, Browser)>,
    pub status: Option<(String, bool)>,
    pub filter: Filter,
    pub search: String,
    pub checked: Option<Instant>,
    pub removing: Option<usize>,
    pub keep_on_remove: bool,
    pub keep_source: bool,
    pub settings_open: bool,
    pub work: (std::sync::mpsc::Sender<(String, bool)>, std::sync::mpsc::Receiver<(String, bool)>),
    pub agent: crate::agent::Page,
    pub reader: crate::docs::Reader,
}

impl Hub {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Hub {
        egui_bench::install(&cc.egui_ctx);
        egui_extras::install_image_loaders(&cc.egui_ctx);
        install::sweep_self();
        let versions = install::versions();
        let slots = TOOLS
            .iter()
            .map(|tool| {
                update::sweep(&install::home(tool));
                let recent = cc
                    .storage
                    .and_then(|s| s.get_string(&recent_key(tool)))
                    .map(|s| s.lines().map(PathBuf::from).collect())
                    .unwrap_or_default();
                let mut slot = Slot {
                    tool,
                    exe: None,
                    managed: false,
                    updater: Updater::cached(tool.repo, None, install::cache()),
                    settled: true,
                    running: Vec::new(),
                    recent,
                    skills: Ok(Vec::new()),
                };
                relocate(&mut slot, &versions, &cc.egui_ctx);
                slot
            })
            .collect();
        let suite =
            Updater::cached(SUITE.repo, Some(env!("CARGO_PKG_VERSION").into()), install::cache());
        let mut hub = Hub {
            slots,
            suite,
            target: Target::host(),
            picking: None,
            status: None,
            filter: Filter::All,
            search: String::new(),
            checked: None,
            removing: None,
            keep_on_remove: true,
            keep_source: cc
                .storage
                .and_then(|s| s.get_string(KEEP_SOURCE_KEY))
                .is_none_or(|v| v != "0"),
            settings_open: false,
            work: std::sync::mpsc::channel(),
            reader: crate::docs::Reader::default(),
            agent: crate::agent::new(
                &cc.egui_ctx,
                cc.storage.and_then(|s| s.get_string(AGENT_OPEN_KEY)).is_some_and(|v| v == "1"),
            ),
        };
        hub.check(&cc.egui_ctx);
        hub
    }

    pub fn check_failure(&self) -> Option<String> {
        self.slots.iter().find_map(|s| match s.updater.state() {
            State::Failed(e) => Some(e),
            _ => None,
        })
    }

    pub fn checking(&self) -> bool {
        matches!(self.suite.state(), State::Checking)
            || self.slots.iter().any(|s| matches!(s.updater.state(), State::Checking))
    }

    pub fn pending(&self) -> usize {
        self.slots.iter().filter(|s| s.wants_install(self.target)).count()
    }

    pub fn count(&self, filter: Filter) -> usize {
        self.slots.iter().filter(|s| filter.admits(s)).count()
    }

    pub fn matches(&self, slot: &Slot) -> bool {
        let q = self.search.trim().to_lowercase();
        self.filter.admits(slot)
            && (q.is_empty()
                || [slot.tool.title, slot.tool.tagline, slot.tool.blurb, slot.tool.group.label()]
                    .iter()
                    .any(|t| t.to_lowercase().contains(&q)))
    }

    pub(crate) fn check(&mut self, ctx: &egui::Context) {
        self.suite.check(ctx);
        for slot in &self.slots {
            slot.updater.check(ctx);
        }
        self.checked = Some(Instant::now());
    }

    fn install(&mut self, ctx: &egui::Context, i: usize) {
        if let Err(e) = self.try_install(ctx, i) {
            self.say(&e, true);
        }
    }

    pub(crate) fn try_install(&mut self, ctx: &egui::Context, i: usize) -> Result<(), String> {
        let target = self.target.ok_or("no builds are published for this machine")?;
        let slot = &mut self.slots[i];
        let tool = slot.tool;
        let Some((release, package, asset)) = slot.choice(Some(target)) else {
            return Err(format!("{} has no build to install for this machine", tool.title));
        };
        slot.settled = false;
        self.status = None;
        let version = release.version.clone();
        let keep = self.keep_source;
        let tx = self.work.0.clone();
        let kept_release = release.clone();
        slot.updater.install(ctx, &release, &asset, move |file| {
            install::place(tool, target, &package, &version, file)?;
            if keep && let Some(warning) = install::keep_source(tool, &kept_release) {
                let _ = tx.send((warning, true));
            }
            Ok(())
        });
        Ok(())
    }

    fn update_self(&mut self, ctx: &egui::Context) {
        let Some(target) = self.target else { return };
        let State::Newer(release) = self.suite.state() else { return };
        let Some(package) = SUITE.package(target, &release.tag) else { return };
        let Some(asset) = release.asset(&package.name).cloned() else {
            self.say(&format!("{} has no {}", release.tag, package.name), true);
            return;
        };
        let keep = self.keep_source;
        let tx = self.work.0.clone();
        self.suite.install(ctx, &release.clone(), &asset, move |file| {
            install::place_self(target, file)?;
            if keep && let Some(warning) = install::keep_source(&SUITE, &release) {
                let _ = tx.send((warning, true));
            }
            Ok(())
        });
    }

    fn uninstall(&mut self, ctx: &egui::Context, i: usize, keep: bool) {
        let tool = self.slots[i].tool;
        match install::remove(tool, keep) {
            Ok(()) => {
                let versions = install::versions();
                relocate(&mut self.slots[i], &versions, ctx);
                self.say(&format!("removed {}", tool.title), false);
            }
            Err(e) => self.say(&format!("could not remove {}: {e}", tool.title), true),
        }
    }

    pub fn source_tag(&self, i: usize) -> Option<String> {
        let tool = self.slots[i].tool;
        let home = install::home(tool);
        let current = self.slots[i].updater.current();
        let by_version = current.and_then(|v| {
            source::kept(&home, tool.name).into_iter().find(|k| k.tag.trim_start_matches('v') == v)
        });
        by_version.or_else(|| source::newest(&home, tool.name)).map(|k| k.tag)
    }

    fn background(
        &self,
        ctx: &egui::Context,
        job: impl FnOnce() -> (String, bool) + Send + 'static,
    ) {
        let tx = self.work.0.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(job());
            ctx.request_repaint();
        });
    }

    fn source_open(&mut self, i: usize) {
        let tool = self.slots[i].tool;
        let Some(tag) = self.source_tag(i) else {
            return self.say("no source is kept yet, use re-download", true);
        };
        let home = install::home(tool);
        match source::extract(&home, tool.name, &tag) {
            Ok(dir) => {
                let origin = source::find(&home, tool.name, &tag).map(|k| k.origin);
                let mut said = format!("source is in {}", dir.display());
                for w in source::warnings(&dir, origin.unwrap_or(source::Origin::Archive)) {
                    said.push_str(&format!(". Warning: {w}"));
                }
                let notes = source::build_notes(&dir);
                if let Some(first) = notes.lines().next() {
                    said.push_str(&format!(". Build with: {first}"));
                }
                self.say(&said, false);
                self.reveal(&dir);
            }
            Err(e) => self.say(&format!("could not extract the source: {e}"), true),
        }
    }

    fn source_fetch(&mut self, i: usize, ctx: &egui::Context) {
        let slot = &self.slots[i];
        let tool = slot.tool;
        let tag = match source::retry(&install::home(tool)) {
            Some((tag, _)) => tag,
            None => match slot.updater.current() {
                Some(v) => format!("v{}", v.trim_start_matches('v')),
                None => return self.say("install the tool first", true),
            },
        };
        let release = [slot.updater.state().release().cloned()]
            .into_iter()
            .flatten()
            .find(|r| r.tag == tag)
            .unwrap_or_else(|| Release {
                version: tag.trim_start_matches('v').to_string(),
                tag: tag.clone(),
                page: String::new(),
                published: String::new(),
                assets: Vec::new(),
            });
        self.say(&format!("downloading the source of {} {tag}", tool.title), false);
        self.background(ctx, move || match install::keep_source(tool, &release) {
            None => (format!("kept the source of {} {tag}", tool.title), false),
            Some(e) => (e, true),
        });
    }

    fn source_vendor(&mut self, i: usize, ctx: &egui::Context) {
        let tool = self.slots[i].tool;
        let Some(tag) = self.source_tag(i) else {
            return self.say("no source is kept yet, use re-download", true);
        };
        self.say(
            &format!("vendoring the dependencies of {}, this can take a while", tool.title),
            false,
        );
        self.background(ctx, move || {
            let home = install::home(tool);
            match source::extract(&home, tool.name, &tag).and_then(|dir| source::vendor(&dir)) {
                Ok(()) => (format!("{} builds offline from {}", tool.title, tag), false),
                Err(e) => (format!("could not vendor {}: {e}", tool.title), true),
            }
        });
    }

    pub fn say(&mut self, msg: &str, fault: bool) {
        self.status = Some((msg.to_string(), fault));
    }

    pub fn reveal(&mut self, path: &Path) {
        if let Err(e) = install::reveal(path) {
            self.say(&format!("could not open the folder: {e}"), true);
        }
    }

    pub fn launch(&mut self, i: usize, path: Option<PathBuf>) {
        match self.try_launch(i, path) {
            Ok(what) => self.say(&what, false),
            Err(e) => self.say(&e, true),
        }
    }

    pub(crate) fn try_launch(&mut self, i: usize, path: Option<PathBuf>) -> Result<String, String> {
        let slot = &mut self.slots[i];
        let title = slot.tool.title;
        let Some(exe) = slot.exe.clone() else {
            return Err(format!("{title} is not installed"));
        };
        let path = path.map(|p| std::fs::canonicalize(&p).unwrap_or(p));
        let mut cmd = Command::new(&exe);
        let log = install::log(slot.tool);
        let stderr = log
            .parent()
            .and_then(|d| std::fs::create_dir_all(d).ok())
            .and_then(|()| std::fs::File::create(&log).ok())
            .map(Stdio::from)
            .unwrap_or_else(Stdio::null);
        cmd.args(slot.tool.args(path.as_deref()))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(stderr);
        if let Some(dir) = working_dir(path.as_deref()) {
            cmd.current_dir(dir);
        }
        no_console(&mut cmd);
        let child = cmd.spawn().map_err(|e| format!("{title} would not start: {e}"))?;
        slot.running.push(child);
        let what = match &path {
            Some(p) => format!("opened {} in {title}", file_label(p)),
            None => format!("started {title}"),
        };
        if let Some(p) = path {
            slot.recent.retain(|r| r != &p);
            slot.recent.insert(0, p);
            slot.recent.truncate(RECENT_MAX);
        }
        Ok(what)
    }

    pub fn reap(&mut self, ctx: &egui::Context) {
        while let Ok((msg, fault)) = self.work.1.try_recv() {
            self.say(&msg, fault);
        }
        let versions = install::versions();
        let mut said = None;
        let mut installed = Vec::new();
        for slot in &mut self.slots {
            if !slot.settled && !slot.updater.install_state().busy() {
                slot.settled = true;
                match slot.updater.install_state() {
                    Install::Done(_) => {
                        relocate(slot, &versions, ctx);
                        let v = slot.updater.current().unwrap_or_default();
                        installed.push(format!("{} {v}", slot.tool.title));
                    }
                    Install::Failed(e) => said = Some(format!("{}: {e}", slot.tool.title)),
                    _ => {}
                }
            }
            let tool = slot.tool;
            slot.running.retain_mut(|child| match child.try_wait() {
                Ok(None) => true,
                Ok(Some(code)) => {
                    if !code.success() {
                        let why =
                            last_words(&install::log(tool)).unwrap_or_else(|| code.to_string());
                        said = Some(format!("{} stopped: {why}", tool.title));
                    }
                    false
                }
                Err(_) => false,
            });
        }
        if !installed.is_empty() {
            let earlier = match &self.status {
                Some((msg, false)) => msg.strip_prefix("installed ").map(|m| format!("{m}, ")),
                _ => None,
            };
            let msg = format!("installed {}{}", earlier.unwrap_or_default(), installed.join(", "));
            self.say(&msg, false);
        }
        if let Some(msg) = said {
            self.say(&msg, true);
        }
    }

    pub fn dropped(&mut self, ctx: &egui::Context) {
        let files: Vec<PathBuf> =
            ctx.input(|i| i.raw.dropped_files.iter().map(|f| f.path().to_path_buf()).collect());
        for path in files {
            match self.slot_for(&path) {
                Some(i) => self.launch(i, Some(path)),
                None => self.say(&format!("no tool here opens {}", file_label(&path)), true),
            }
        }
    }

    pub fn slot_for(&self, path: &Path) -> Option<usize> {
        let tool = for_path(path)?;
        self.slots.iter().position(|s| std::ptr::eq(s.tool, tool))
    }

    pub fn browse(&mut self, ctx: &egui::Context) {
        let Some((i, browser)) = &mut self.picking else { return };
        let i = *i;
        match browser.show(ctx) {
            egui_bench::prelude::Picked::Browsing => {}
            egui_bench::prelude::Picked::Cancelled => self.picking = None,
            egui_bench::prelude::Picked::File(p) | egui_bench::prelude::Picked::Folder(p) => {
                self.picking = None;
                self.launch(i, Some(p));
            }
        }
    }

    pub fn apply(&mut self, ctx: &egui::Context, acts: Vec<Act>) {
        for act in acts {
            match act {
                Act::Launch(i, path) => self.launch(i, path),
                Act::Forget(i) => self.slots[i].recent.clear(),
                Act::Install(i) => self.install(ctx, i),
                Act::InstallAll => {
                    for i in 0..self.slots.len() {
                        if self.slots[i].wants_install(self.target) {
                            self.install(ctx, i);
                        }
                    }
                }
                Act::Uninstall(i) => self.removing = Some(i),
                Act::Remove(i, keep) => self.uninstall(ctx, i, keep),
                Act::SourceOpen(i) => self.source_open(i),
                Act::SourceFetch(i) => self.source_fetch(i, ctx),
                Act::SourceVendor(i) => self.source_vendor(i, ctx),
                Act::Settings => self.settings_open = !self.settings_open,
                Act::Reveal(i) => {
                    if let Some(exe) = self.slots[i].exe.clone() {
                        self.reveal(&exe);
                    }
                }
                Act::RevealPath(path) => self.reveal(&path),
                Act::Check => self.check(ctx),
                Act::UpdateSelf => self.update_self(ctx),
                Act::Restart => match update::relaunch() {
                    Ok(()) => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                    Err(e) => self.say(&format!("could not restart: {e}"), true),
                },
                Act::Url(url) => ctx.open_url(egui::OpenUrl::new_tab(url)),
                Act::Pick(i) => self.pick(i),
                Act::Page(i, tab) => {
                    let repo = self.slots[i].tool.repo;
                    self.reader.show(repo, tab);
                }
                Act::SuitePage(tab) => self.reader.show(SUITE.repo, tab),
                Act::ShowAll => {
                    self.filter = Filter::All;
                    self.search.clear();
                }
            }
        }
    }

    fn pick(&mut self, i: usize) {
        let slot = &self.slots[i];
        let start = slot
            .recent
            .iter()
            .find_map(
                |p| if p.is_dir() { Some(p.clone()) } else { p.parent().map(Path::to_path_buf) },
            )
            .filter(|d| d.is_dir())
            .or_else(std::env::home_dir)
            .unwrap_or_else(|| PathBuf::from("."));
        let title = format!("open in {}", slot.tool.title);
        let browser = Browser::new(&title, start)
            .suffixes(slot.tool.suffixes())
            .folders(slot.tool.takes_folders());
        self.picking = Some((i, browser));
    }

    pub fn save_recent(&self, storage: &mut dyn eframe::Storage) {
        storage.set_string(KEEP_SOURCE_KEY, if self.keep_source { "1" } else { "0" }.into());
        storage.set_string(AGENT_OPEN_KEY, if self.agent.open { "1" } else { "0" }.into());
        for slot in &self.slots {
            let list: Vec<String> =
                slot.recent.iter().map(|p| p.to_string_lossy().into_owned()).collect();
            storage.set_string(&recent_key(slot.tool), list.join("\n"));
        }
    }
}

fn relocate(slot: &mut Slot, versions: &[(String, String)], ctx: &egui::Context) {
    let tool = slot.tool;
    // what the suite put there: a system install, or a copy in its own folder
    let managed = install::installed(tool).or_else(|| install::portable(tool));
    slot.managed = managed.is_some();
    slot.exe = managed.or_else(|| locate(&search_dirs(), tool.bin));
    slot.skills = Ok(Vec::new());
    let recorded = versions.iter().find(|(n, _)| n == tool.name).map(|(_, v)| v.clone());
    if slot.managed {
        if let (Some(exe), Some(version)) = (&slot.exe, &recorded) {
            slot.skills = sync_skill(tool, exe, version);
        }
        if let Some(exe) = install::portable(tool) {
            // copies kept in the suite's folder get a menu entry too
            std::thread::spawn(move || {
                if let Err(e) = launcher::ensure(tool, &exe) {
                    eprintln!("{}: {e}", tool.name);
                }
            });
        }
    }
    match (&slot.exe, recorded) {
        (Some(_), Some(v)) if slot.managed => slot.updater.set_current(Some(v)),
        (Some(exe), _) if tool.version_flag => {
            let exe = exe.clone();
            let updater = slot.updater.clone();
            let ctx = ctx.clone();
            std::thread::spawn(move || {
                updater.set_current(update::probe_version(&exe));
                ctx.request_repaint();
            });
        }
        _ => slot.updater.set_current(None),
    }
}

fn sync_skill(tool: &Tool, exe: &Path, version: &str) -> Result<Vec<Placed>, String> {
    if !skills::current(tool, version) {
        skills::install(tool, exe, version)?;
    }
    Ok(skills::placed(tool))
}

fn search_dirs() -> Vec<PathBuf> {
    let beside = std::env::current_exe()
        .ok()
        .and_then(|e| std::fs::canonicalize(e).ok())
        .and_then(|e| e.parent().map(Path::to_path_buf));
    let extra = std::env::var_os(TOOLS_ENV)
        .map(|v| std::env::split_paths(&v).collect::<Vec<_>>())
        .unwrap_or_default();
    beside.into_iter().chain(extra).collect()
}

fn recent_key(tool: &Tool) -> String {
    format!("recent.{}", tool.name)
}

fn last_words(log: &Path) -> Option<String> {
    let text = std::fs::read_to_string(log).ok()?;
    let line = text
        .lines()
        .map(str::trim)
        .rfind(|l| !l.is_empty() && !l.starts_with("note:") && !l.starts_with("thread '"))?;
    let mut line = line.to_string();
    if line.chars().count() > 140 {
        line = line.chars().take(140).collect::<String>() + "…";
    }
    Some(line)
}

pub fn file_label(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

fn working_dir(path: Option<&Path>) -> Option<PathBuf> {
    match path {
        Some(p) if p.is_dir() => Some(p.to_path_buf()),
        Some(p) => p.parent().map(Path::to_path_buf),
        None => std::env::home_dir(),
    }
}

#[cfg(windows)]
fn no_console(cmd: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    cmd.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn no_console(_cmd: &mut Command) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_crash_is_reported_by_its_message() {
        let log = std::env::temp_dir().join(format!("bench-last-words-{}", std::process::id()));
        std::fs::write(
            &log,
            "\nthread 'main' (1) panicked at crates/x/src/levels.rs:228:36:\nno entry found for key\nnote: run with `RUST_BACKTRACE=1`\n",
        )
        .unwrap();
        assert_eq!(last_words(&log).as_deref(), Some("no entry found for key"));
        std::fs::remove_file(&log).unwrap();
    }
}
