use super::tools::{text, tool_of};
use crate::catalog::{TOOLS, Target, Tool};
use egui_bench::agent::Cx;
use egui_bench::agent::grants::Access;
use egui_bench::agent::local::{execute, report};
use egui_bench::agent::workspace::Workspace;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const DEFAULT_TIMEOUT: u64 = 120;
const MAX_TIMEOUT: u64 = 900;

pub fn launch(args: &Value, cx: &Cx) -> Result<String, String> {
    let tool = tool_of(args)?;
    let path = match text(args, "path") {
        Some(p) => Some(cx.workspace.resolve(&p)?.display().to_string()),
        None => None,
    };
    cx.ask("launch", serde_json::json!({ "tool": tool, "path": path })).map(|v| v.to_string())
}

pub fn run_tool(args: &Value, cx: &Cx) -> Result<String, String> {
    let name = tool_of(args)?;
    let tool = TOOLS
        .iter()
        .find(|t| t.name == name.trim().to_lowercase())
        .ok_or_else(|| format!("no tool called {name:?}"))?;
    let argv: Vec<String> = match args.get("args") {
        Some(Value::Array(items)) => items
            .iter()
            .map(|v| match v {
                Value::String(s) => Ok(s.clone()),
                Value::Number(n) => Ok(n.to_string()),
                Value::Bool(b) => Ok(b.to_string()),
                _ => Err("`args` is a list of strings".to_string()),
            })
            .collect::<Result<_, _>>()?,
        Some(Value::Null) | None => Vec::new(),
        Some(_) => return Err("`args` is a list of strings".into()),
    };
    vet(tool, &argv, cx.workspace)?;
    let exe = cx.ask("path", serde_json::json!({ "tool": tool.name }))?;
    let exe = exe
        .get("path")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .ok_or_else(|| format!("{} is not installed. Install it first", tool.title))?;
    let program = program_of(tool, &exe);
    let timeout = args
        .get("timeout_s")
        .and_then(Value::as_u64)
        .unwrap_or(DEFAULT_TIMEOUT)
        .clamp(1, MAX_TIMEOUT);
    let stdin = text(args, "stdin");
    let started = Instant::now();
    let out = execute(
        &program,
        &argv,
        cx.workspace.root(),
        stdin.as_deref(),
        Duration::from_secs(timeout),
        cx.stop,
    )?;
    report(format!("$ {} {}\n", tool.cli_bin_or_name(), argv.join(" ")), out, started, timeout)
}

impl Tool {
    pub fn cli_bin_or_name(&self) -> &'static str {
        if self.cli_bin.is_empty() { self.name } else { self.cli_bin }
    }
}

pub fn program_of(tool: &Tool, exe: &Path) -> PathBuf {
    if tool.cli_bin.is_empty() {
        return exe.to_path_buf();
    }
    let name = match Target::host() {
        Some(t) => t.exe(tool.cli_bin),
        None => tool.cli_bin.to_string(),
    };
    match exe.parent().map(|d| d.join(&name)) {
        Some(beside) if beside.is_file() => beside,
        _ => PathBuf::from(name),
    }
}

pub fn vet(tool: &Tool, argv: &[String], ws: &Workspace) -> Result<(), String> {
    if tool.cli.is_empty() {
        return Err(format!(
            "{} has no command line the agent can run. launch it and the user works in its window",
            tool.title
        ));
    }
    let Some(first) = argv.first() else {
        return Err(format!("give a subcommand: {}", tool.cli.join(", ")));
    };
    if !tool.cli.contains(&first.as_str()) {
        return Err(format!(
            "{first:?} is not a command the agent can run for {}. One of: {}",
            tool.title,
            tool.cli.join(", ")
        ));
    }
    for arg in argv {
        if arg == "--remote" {
            return Err("--remote spends the user's money and is not allowed".into());
        }
        let value = arg.split_once('=').map(|(_, v)| v).unwrap_or(arg);
        if value.starts_with('~') {
            return Err(format!(
                "{arg:?}: write the full path, which request_access printed, not ~"
            ));
        }
        ws.resolve_for(value, Access::Write).map_err(|_| {
            format!(
                "{arg:?} is outside the workspace and not in a folder the user shared for \
                 writing. run_tool cannot honour read only, so use bash to read from a read only \
                 folder, or request_access with write=true"
            )
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tool(name: &str) -> &'static Tool {
        TOOLS.iter().find(|t| t.name == name).unwrap()
    }

    fn v(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|s| s.to_string()).collect()
    }

    fn space(name: &str) -> (Workspace, PathBuf) {
        let base = std::env::temp_dir().join(format!("suite-vet-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("work")).unwrap();
        std::fs::create_dir_all(base.join("shared")).unwrap();
        let ws = Workspace::open(&base.join("work")).unwrap();
        (ws, base.join("shared").canonicalize().unwrap())
    }

    #[test]
    fn only_a_headless_subcommand_is_allowed() {
        let (ws, _) = space("sub");
        let ok = |t: &str, a: &[&str]| vet(tool(t), &v(a), &ws).is_ok();
        assert!(ok("gcad", &["check", "plate.gcad"]));
        assert!(!ok("gcad", &["plate.gcad"]));
        assert!(!ok("gcad", &["view", "plate.gcad"]));
        assert!(!ok("gcad", &[]));
        assert!(!ok("agentee", &["view"]));
        assert!(!ok("agentee", &["mcp"]));
        assert!(ok("agentee", &["edit", "sch", "n", "-"]));
        assert!(!ok("waveshark", &["--help"]));
        assert!(!ok("cycler", &["charge"]));
        assert!(ok("cycler", &["profile", "--chemistry", "li-ion"]));
    }

    #[test]
    fn a_path_cannot_leave_the_workspace_through_an_argument() {
        let (ws, _) = space("leave");
        let ok = |t: &str, a: &[&str]| vet(tool(t), &v(a), &ws).is_ok();
        assert!(!ok("gcad", &["export", "a.gcad", "/tmp/a.step"]));
        assert!(!ok("gcad", &["export", "a.gcad", "../a.step"]));
        assert!(!ok("gcad", &["export", "a.gcad", "~/a.step"]));
        assert!(!ok("agentee", &["render", "x", "-o=/tmp/x.png"]));
        assert!(ok("gcad", &["check", "a.gasm", "--set", "front=14"]));
        assert!(!ok("gcad", &["sim", "a.gsim", "--remote"]));
    }

    #[test]
    fn a_folder_shared_for_writing_is_a_path_run_tool_accepts_and_read_only_is_not() {
        let (ws, shared) = space("shared");
        let part = shared.join("part.gcad");
        let part = part.to_str().unwrap();
        let check = || vet(tool("gcad"), &v(&["check", part]), &ws);
        assert!(check().unwrap_err().contains("not in a folder the user shared"));
        ws.grants().grant(shared.clone(), false);
        assert!(check().unwrap_err().contains("read only"), "read only is not enough to run in");
        ws.grants().grant(shared.clone(), true);
        assert!(check().is_ok());
        let out = format!("-o={}", shared.join("out.step").display());
        assert!(vet(tool("gcad"), &v(&["export", "a.gcad", &out]), &ws).is_ok());
        let up = format!("{}/../elsewhere.step", shared.display());
        assert!(vet(tool("gcad"), &v(&["export", "a.gcad", &up]), &ws).is_err());
    }

    #[test]
    fn the_cycler_command_line_is_its_own_binary() {
        let p = program_of(tool("cycler"), Path::new("/nowhere/cycler-ui"));
        assert_eq!(p, PathBuf::from("cycler"));
        let p = program_of(tool("gcad"), Path::new("/usr/local/bin/gcad"));
        assert_eq!(p, PathBuf::from("/usr/local/bin/gcad"));
    }
}
