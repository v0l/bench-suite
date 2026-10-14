use crate::catalog::Tool;
use egui_bench::update::replace;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const MARK: &str = ".bench-suite";

pub const SHARED_READERS: [&str; 6] =
    ["Codex", "Cursor", "OpenCode", "Gemini CLI", "GitHub Copilot", "pi"];

#[derive(Clone, Copy)]
enum Base {
    Home,
    Config,
    Claude,
}

struct Linked {
    title: &'static str,
    base: Base,
    detect: &'static str,
    skills: &'static str,
}

const LINKED: [Linked; 6] = [
    Linked { title: "Claude Code", base: Base::Claude, detect: "", skills: "skills" },
    Linked {
        title: "Windsurf",
        base: Base::Home,
        detect: ".codeium/windsurf",
        skills: ".codeium/windsurf/skills",
    },
    Linked { title: "Goose", base: Base::Config, detect: "goose", skills: "goose/skills" },
    Linked { title: "Roo Code", base: Base::Home, detect: ".roo", skills: ".roo/skills" },
    Linked { title: "Kiro", base: Base::Home, detect: ".kiro", skills: ".kiro/skills" },
    Linked { title: "Qwen Code", base: Base::Home, detect: ".qwen", skills: ".qwen/skills" },
];

pub struct Layout {
    pub shared: PathBuf,
    pub links: Vec<(&'static str, PathBuf)>,
}

impl Layout {
    pub fn detect() -> Option<Layout> {
        let home = dirs::home_dir()?;
        let from_env =
            |name: &str| std::env::var_os(name).filter(|v| !v.is_empty()).map(PathBuf::from);
        let config = from_env("XDG_CONFIG_HOME").unwrap_or_else(|| home.join(".config"));
        let claude = from_env("CLAUDE_CONFIG_DIR").unwrap_or_else(|| home.join(".claude"));
        Some(Layout::detect_in(&home, &config, &claude))
    }

    fn detect_in(home: &Path, config: &Path, claude: &Path) -> Layout {
        let links = LINKED
            .iter()
            .filter_map(|agent| {
                let base = match agent.base {
                    Base::Home => home,
                    Base::Config => config,
                    Base::Claude => claude,
                };
                let found = if agent.detect.is_empty() {
                    base.to_path_buf()
                } else {
                    base.join(agent.detect)
                };
                found.is_dir().then(|| (agent.title, base.join(agent.skills)))
            })
            .collect();
        Layout { shared: home.join(".agents").join("skills"), links }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum How {
    Copied,
    Linked,
    Theirs,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Placed {
    pub agent: Option<&'static str>,
    pub path: PathBuf,
    pub how: How,
}

pub fn source(tool: &Tool, exe: &Path) -> Option<PathBuf> {
    let name = tool.skill?;
    let dir = exe.parent()?.join("skills").join(name);
    dir.join("SKILL.md").is_file().then_some(dir)
}

pub fn install(tool: &Tool, exe: &Path, version: &str) -> Result<Vec<Placed>, String> {
    match (source(tool, exe), Layout::detect()) {
        (Some(from), Some(layout)) => install_into(&from, version, &layout),
        _ => Ok(Vec::new()),
    }
}

pub fn remove(tool: &Tool) -> Result<(), String> {
    match (tool.skill, Layout::detect()) {
        (Some(name), Some(layout)) => remove_from(name, &layout),
        _ => Ok(()),
    }
}

pub fn placed(tool: &Tool) -> Vec<Placed> {
    match (tool.skill, Layout::detect()) {
        (Some(name), Some(layout)) => placed_in(name, &layout),
        _ => Vec::new(),
    }
}

pub fn current(tool: &Tool, version: &str) -> bool {
    let (Some(name), Some(layout)) = (tool.skill, Layout::detect()) else { return true };
    let placed = placed_in(name, &layout);
    placed.len() == layout.links.len() + 1
        && placed.iter().all(|p| p.how != How::Copied || mark(&p.path).as_deref() == Some(version))
}

pub fn readers(placed: &[Placed]) -> String {
    let linked = placed.iter().filter(|p| p.how != How::Theirs).filter_map(|p| p.agent);
    SHARED_READERS.iter().copied().chain(linked).collect::<Vec<_>>().join(", ")
}

fn mark(dest: &Path) -> Option<String> {
    fs::read_to_string(dest.join(MARK)).ok().map(|v| v.trim().to_string())
}

fn ours(dest: &Path) -> bool {
    fs::symlink_metadata(dest).is_ok_and(|m| m.is_dir() && !m.file_type().is_symlink())
        && dest.join(MARK).is_file()
}

fn same(a: &Path, b: &Path) -> bool {
    matches!((fs::canonicalize(a), fs::canonicalize(b)), (Ok(a), Ok(b)) if a == b)
}

fn points_at(link: &Path, target: &Path) -> bool {
    let Some(dir) = link.parent() else { return false };
    fs::symlink_metadata(link).is_ok_and(|m| m.file_type().is_symlink())
        && fs::read_link(link).is_ok_and(|t| lexical(&dir.join(t)) == lexical(target))
}

fn lexical(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut out = PathBuf::new();
    for part in path.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

#[cfg(unix)]
fn relative(from_dir: &Path, to: &Path) -> PathBuf {
    let real = |p: &Path| fs::canonicalize(p).unwrap_or_else(|_| lexical(p));
    let to = match (to.parent(), to.file_name()) {
        (Some(parent), Some(name)) => real(parent).join(name),
        _ => lexical(to),
    };
    let from = real(from_dir);
    let shared = from.components().zip(to.components()).take_while(|(a, b)| a == b).count();
    let ups = from.components().count() - shared;
    std::iter::repeat_n(std::path::Component::ParentDir.as_os_str(), ups)
        .map(PathBuf::from)
        .chain(to.components().skip(shared).map(|c| PathBuf::from(c.as_os_str())))
        .collect()
}

fn placed_in(name: &str, layout: &Layout) -> Vec<Placed> {
    let shared = layout.shared.join(name);
    let mut placed = Vec::new();
    if fs::symlink_metadata(&shared).is_ok() {
        let how = if ours(&shared) { How::Copied } else { How::Theirs };
        placed.push(Placed { agent: None, path: shared.clone(), how });
    }
    for (title, dir) in &layout.links {
        let dest = dir.join(name);
        let how = if same(&dest, &shared) {
            How::Linked
        } else if ours(&dest) {
            How::Copied
        } else if fs::symlink_metadata(&dest).is_ok() && !points_at(&dest, &shared) {
            How::Theirs
        } else {
            continue;
        };
        placed.push(Placed { agent: Some(title), path: dest, how });
    }
    placed
}

fn install_into(from: &Path, version: &str, layout: &Layout) -> Result<Vec<Placed>, String> {
    let name = from.file_name().ok_or("the skill folder has no name")?.to_string_lossy();
    let shared = layout.shared.join(name.as_ref());
    if fs::symlink_metadata(&shared).is_ok() && !ours(&shared) {
        return Ok(placed_in(&name, layout));
    }
    copy_marked(from, &shared, version)?;
    for (_, dir) in &layout.links {
        let dest = dir.join(name.as_ref());
        if same(&dest, &shared) {
            continue;
        }
        let stale = points_at(&dest, &shared) || ours(&dest);
        if fs::symlink_metadata(&dest).is_ok() && !stale {
            continue;
        }
        if stale {
            unlink(&dest).map_err(|e| format!("{}: {e}", dest.display()))?;
        }
        fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        if link(&shared, &dest).is_err() {
            copy_marked(from, &dest, version)?;
        }
    }
    Ok(placed_in(&name, layout))
}

fn remove_from(name: &str, layout: &Layout) -> Result<(), String> {
    let shared = layout.shared.join(name);
    let owned = ours(&shared);
    for (_, dir) in &layout.links {
        let dest = dir.join(name);
        let gone = if owned && points_at(&dest, &shared) {
            unlink(&dest)
        } else if ours(&dest) {
            fs::remove_dir_all(&dest)
        } else {
            Ok(())
        };
        gone.map_err(|e| format!("{}: {e}", dest.display()))?;
    }
    if ours(&shared) {
        fs::remove_dir_all(&shared).map_err(|e| format!("{}: {e}", shared.display()))?;
    }
    Ok(())
}

fn copy_marked(from: &Path, dest: &Path, version: &str) -> Result<(), String> {
    let (Some(home), Some(name)) = (dest.parent(), dest.file_name()) else {
        return Err(format!("nowhere to put {}", dest.display()));
    };
    fs::create_dir_all(home).map_err(|e| format!("{}: {e}", home.display()))?;
    let staging = home.join(format!(".{}.copy", name.to_string_lossy()));
    let _ = fs::remove_dir_all(&staging);
    copy_dir(from, &staging)
        .and_then(|()| {
            fs::write(staging.join(MARK), format!("{version}\n")).map_err(|e| e.to_string())
        })
        .and_then(|()| replace(&staging, dest))
        .map_err(|e| {
            let _ = fs::remove_dir_all(&staging);
            format!("could not install the agent skill in {}: {e}", home.display())
        })
}

fn copy_dir(from: &Path, to: &Path) -> Result<(), String> {
    fs::create_dir_all(to).map_err(|e| format!("{}: {e}", to.display()))?;
    for entry in fs::read_dir(from).map_err(|e| format!("{}: {e}", from.display()))?.flatten() {
        let (src, dest) = (entry.path(), to.join(entry.file_name()));
        if src.is_dir() {
            copy_dir(&src, &dest)?;
        } else {
            fs::copy(&src, &dest).map_err(|e| format!("{}: {e}", src.display()))?;
        }
    }
    Ok(())
}

#[cfg(unix)]
fn link(target: &Path, at: &Path) -> io::Result<()> {
    let dir = at.parent().ok_or_else(|| io::Error::other("no folder to link in"))?;
    std::os::unix::fs::symlink(relative(dir, target), at)
}

#[cfg(windows)]
fn link(target: &Path, at: &Path) -> io::Result<()> {
    std::os::windows::fs::symlink_dir(target, at)
}

fn unlink(path: &Path) -> io::Result<()> {
    fs::remove_file(path).or_else(|_| fs::remove_dir(path))
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Scratch {
            let dir =
                std::env::temp_dir().join(format!("bench-skills-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            Scratch(dir)
        }

        fn release(&self, text: &str) -> PathBuf {
            let skill = self.0.join("tool").join("skills").join("gcad");
            fs::create_dir_all(skill.join("refs")).unwrap();
            fs::write(skill.join("SKILL.md"), text).unwrap();
            fs::write(skill.join("refs").join("format.md"), "ops").unwrap();
            skill
        }

        fn layout(&self) -> Layout {
            Layout {
                shared: self.0.join("home/.agents/skills"),
                links: vec![("Claude Code", self.0.join("home/.claude/skills"))],
            }
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn only_agents_whose_folder_exists_are_linked() {
        let s = Scratch::new("detect");
        let home = s.0.join("home");
        fs::create_dir_all(home.join(".claude")).unwrap();
        fs::create_dir_all(home.join(".codeium/windsurf")).unwrap();
        fs::create_dir_all(home.join(".config/goose")).unwrap();
        let layout = Layout::detect_in(&home, &home.join(".config"), &home.join(".claude"));
        assert_eq!(layout.shared, home.join(".agents/skills"));
        assert_eq!(
            layout.links,
            [
                ("Claude Code", home.join(".claude/skills")),
                ("Windsurf", home.join(".codeium/windsurf/skills")),
                ("Goose", home.join(".config/goose/skills")),
            ]
        );
    }

    #[test]
    fn the_skill_is_copied_once_and_linked_for_each_agent() {
        let s = Scratch::new("copy");
        let layout = s.layout();
        let from = s.release("first");
        let placed = install_into(&from, "v1", &layout).unwrap();
        let shared = layout.shared.join("gcad");
        let claude = layout.links[0].1.join("gcad");
        assert_eq!(
            placed,
            [
                Placed { agent: None, path: shared.clone(), how: How::Copied },
                Placed { agent: Some("Claude Code"), path: claude.clone(), how: How::Linked },
            ]
        );
        assert_eq!(fs::read_link(&claude).unwrap(), Path::new("../../.agents/skills/gcad"));
        fs::write(from.join("SKILL.md"), "second").unwrap();
        install_into(&from, "v2", &layout).unwrap();
        assert_eq!(fs::read_to_string(claude.join("SKILL.md")).unwrap(), "second");
        assert_eq!(mark(&shared).as_deref(), Some("v2"));
        assert_eq!(fs::read_to_string(claude.join("refs/format.md")).unwrap(), "ops");
        let left: Vec<_> =
            fs::read_dir(&layout.shared).unwrap().flatten().map(|e| e.file_name()).collect();
        assert_eq!(left, ["gcad"]);
    }

    #[test]
    fn skills_someone_put_there_themselves_are_left_alone() {
        let s = Scratch::new("theirs");
        let layout = s.layout();
        let mine = s.0.join("dotfiles/gcad");
        fs::create_dir_all(&mine).unwrap();
        fs::write(mine.join("SKILL.md"), "mine").unwrap();
        fs::create_dir_all(&layout.shared).unwrap();
        std::os::unix::fs::symlink(&mine, layout.shared.join("gcad")).unwrap();
        let claude = layout.links[0].1.join("gcad");
        fs::create_dir_all(&layout.links[0].1).unwrap();
        std::os::unix::fs::symlink("../../.agents/skills/gcad", &claude).unwrap();
        let placed = install_into(&s.release("shipped"), "v1", &layout).unwrap();
        assert_eq!(placed[0].how, How::Theirs);
        remove_from("gcad", &layout).unwrap();
        assert_eq!(fs::read_to_string(mine.join("SKILL.md")).unwrap(), "mine");
        assert_eq!(fs::read_to_string(claude.join("SKILL.md")).unwrap(), "mine");
        assert!(fs::symlink_metadata(&claude).unwrap().file_type().is_symlink());
    }

    #[test]
    fn an_agent_folder_linked_to_the_shared_one_gets_no_second_link() {
        let s = Scratch::new("parent");
        let layout = s.layout();
        fs::create_dir_all(&layout.shared).unwrap();
        fs::create_dir_all(layout.links[0].1.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(&layout.shared, &layout.links[0].1).unwrap();
        let placed = install_into(&s.release("x"), "v1", &layout).unwrap();
        assert_eq!(placed[1].how, How::Linked);
        assert!(
            !fs::symlink_metadata(layout.shared.join("gcad")).unwrap().file_type().is_symlink()
        );
    }

    #[test]
    fn uninstalling_removes_the_copy_and_its_links() {
        let s = Scratch::new("remove");
        let layout = s.layout();
        install_into(&s.release("x"), "v1", &layout).unwrap();
        remove_from("gcad", &layout).unwrap();
        assert!(fs::symlink_metadata(layout.shared.join("gcad")).is_err());
        assert!(fs::symlink_metadata(layout.links[0].1.join("gcad")).is_err());
        assert!(placed_in("gcad", &layout).is_empty());
    }
}
