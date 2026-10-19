use crate::catalog::{TOOLS, Tool};
use egui_bench::agent::skillset::{Skill, listing};
use egui_bench::agent::toolkit::Tool as Entry;
use egui_bench::agent::tools::{object, string};
use serde_json::{Value, json};
use std::path::Path;

const INTRO: &str = "You are the coding agent inside bench suite, a launcher for a set of \
engineering tools. You design with them: you write their files in a workspace folder, check and \
build them by running their command lines, and show the user the result in the tool's own window. \
You also install, update and start the tools.\n\
\n\
How to work:\n\
- Call list_tools before saying what is installed, what version it is or whether an update is \
out. Do not guess. install both installs a missing tool and updates one that is behind, and \
uninstall asks the user to confirm in the window.\n\
- Before writing or editing a tool's files, load its skill with load_skill. The skill is the \
manual: file formats, selectors, workflow and the mistakes to avoid. Do not write these formats \
from memory. load_skill with no name lists the skills, and a skill can have other files you \
read with its file argument.\n\
- Work in the workspace with list_files, read_file, search_files, write_file and edit_file. Read \
a file before you edit it. Paths are relative to the workspace. To use a folder outside it, such as a project or a parts library, call request_access with the folder and a reason: the user chooses read only, read and write, or no. Ask for the narrowest folder and read only unless you must change something. run_tool takes a shared folder's path only when it was shared for writing.\n\
- Check your work with run_tool, which runs a tool's command line in the workspace and returns \
what it printed. Run the check after every change and read the errors it prints. Fix them \
yourself and run it again; do not hand the user a design you have not checked. Use the tool's \
own docs subcommand when you need a reference the skill does not give.\n\
- launch opens the tool's window for the user on a file in the workspace. Do it when there is \
something to look at, not as a step in the work.\n\
- run_tool takes an argument list, not a command line, and only the subcommands listed below. For \
pipes, globs, scripts and anything else use bash, which runs in a sandbox: it reads the system and \
the tools, writes only inside the workspace, and cannot see the rest of the user's files.\n\
- Be brief. Say what you did and what the check said. Write markdown, and put code and file \
contents in fenced blocks.\n\
\n\
The tools follow, one section each.";

fn section(tool: &Tool) -> String {
    let mut out = format!("## {} (name: {})\n{}\n", tool.title, tool.name, tool.tagline);
    out.push_str(tool.guide);
    out.push('\n');
    out.push_str(&match (tool.suffixes(), tool.takes_folders()) {
        ([], false) => "launch: opens no files, launch it bare.".to_string(),
        (suffixes, folders) => {
            let folder = if folders { ", or a project folder" } else { "" };
            format!("launch: can open {}{folder}.", suffixes.join(" "))
        }
    });
    out.push('\n');
    out.push_str(&match tool.cli.is_empty() {
        true => {
            "run_tool: not available. It is a window only, so launch it for the user.".to_string()
        }
        false => format!("run_tool subcommands: {}.", tool.cli.join(", ")),
    });
    out.push('\n');
    if let Some(skill) = tool.skill {
        out.push_str(&format!(
            "skill: `{skill}`. Load it with load_skill before writing this tool's files. It is \
             there once the tool is installed through the suite.\n"
        ));
    }
    out
}

pub fn brief(workspace: &Path, skills: &[Skill]) -> String {
    let mut out = String::from(INTRO);
    for tool in TOOLS.iter() {
        out.push_str("\n\n");
        out.push_str(&section(tool));
    }
    out.push_str(&format!("\n\nThe workspace is {}.\n", workspace.display()));
    out.push_str(&format!("\nSkills installed now, for load_skill:\n{}\n", listing(skills)));
    out
}

pub fn tools() -> Vec<Entry> {
    let tool = string("A tool name from list_tools, such as gcad");
    vec![
        Entry::plain(
            "list_tools",
            "Every tool the suite manages: whether it is installed, its version, the \
             latest release, whether an update is out, what files it opens and how many \
             copies are running. Also the suite's own version.",
            |_, cx| cx.ask("list_tools", Value::Null).map(|v| v.to_string()),
        ),
        Entry::plain(
            "check_updates",
            "Ask GitHub for the newest release of every tool and of the suite, wait for \
             the answer, and return the same list as list_tools.",
            |_, cx| cx.ask("check_updates", Value::Null).map(|v| v.to_string()),
        ),
        Entry::new(
            "install",
            "Download and install a tool, or update it when a newer release is out. \
             Waits until it finishes and reports the result.",
            object(json!({ "tool": tool }), &["tool"]),
            |args, cx| {
                tool_of(args)?;
                cx.ask("install", args.clone()).map(|v| v.to_string())
            },
        )
        .summarised_by(|args| text(args, "tool")),
        Entry::new(
            "uninstall",
            "Remove a tool the suite installed. The user is asked to confirm in the \
             window; the answer says whether they did.",
            object(json!({ "tool": tool }), &["tool"]),
            |args, cx| {
                tool_of(args)?;
                cx.ask("uninstall", args.clone()).map(|v| v.to_string())
            },
        )
        .summarised_by(|args| text(args, "tool")),
        Entry::new(
            "launch",
            "Open a tool's window for the user, optionally on a file or folder it opens. \
             The path is inside the workspace and must be one the tool opens, which \
             list_tools shows. Use this to show the user a result, not to do the work: \
             run_tool does the work.",
            object(
                json!({
                    "tool": tool,
                    "path": string("A file or folder in the workspace to open in the tool"),
                }),
                &["tool"],
            ),
            super::local::launch,
        )
        .summarised_by(|args| {
            Some(format!(
                "{} {}",
                text(args, "tool").unwrap_or_default(),
                text(args, "path").unwrap_or_default()
            ))
        }),
        Entry::new(
            "run_tool",
            "Run an installed tool's command line in the workspace and read what it \
             prints: this is how you check, build, render, export and simulate. Only \
             the headless subcommands listed for each tool in your instructions are \
             allowed, and every path argument must stay inside the workspace. Windows \
             and live hardware are not reachable from here; launch is for those.",
            object(
                json!({
                    "tool": tool,
                    "args": { "type": "array", "items": { "type": "string" },
                              "description": "The arguments, starting with the subcommand, such as [\"check\", \"plate.gcad\"]" },
                    "stdin": string("Text to feed the command, such as a batch for `agentee edit sch NAME -`"),
                    "timeout_s": { "type": "integer", "description": "Seconds before it is killed, default 120, at most 900" },
                }),
                &["tool"],
            ),
            super::local::run_tool,
        )
        .summarised_by(run_tool_summary),
    ]
}

pub fn text(args: &Value, key: &str) -> Option<String> {
    args.get(key).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()).map(String::from)
}

pub fn tool_of(args: &Value) -> Result<String, String> {
    text(args, "tool").ok_or_else(|| "missing `tool`".to_string())
}

fn run_tool_summary(args: &Value) -> Option<String> {
    let tool = text(args, "tool").unwrap_or_default();
    let argv: Vec<String> = args
        .get("args")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect())
        .unwrap_or_default();
    Some(format!("{tool} {}", argv.join(" ")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_bench::agent::{Backend, tools::offered};

    #[test]
    fn every_suite_tool_has_a_name_a_sentence_and_an_object_schema() {
        let kit = super::super::Suite::default().toolkit().clone();
        assert_eq!(kit.len(), 6);
        assert!(kit.problems(&egui_bench::agent::local::NAMES).is_empty());
    }

    #[test]
    fn the_model_is_offered_the_builtin_tools_and_the_suites() {
        let names: Vec<String> =
            offered(&super::super::Suite::default()).into_iter().map(|s| s.name).collect();
        for want in ["load_skill", "read_file", "write_file", "list_tools", "run_tool", "launch"] {
            assert!(names.iter().any(|n| n == want), "{want} is not offered");
        }
        assert!(super::super::Suite::default().toolkit().clone().find("bash").is_none());
    }

    #[test]
    fn the_brief_has_a_section_for_every_tool() {
        let brief = brief(Path::new("/work"), &[]);
        for tool in TOOLS.iter() {
            assert!(tool.guide.len() > 200, "{} has no guide", tool.name);
            assert!(
                brief.contains(&format!("## {} (name: {})", tool.title, tool.name)),
                "{}",
                tool.name
            );
            assert!(brief.contains(tool.guide), "{}", tool.name);
        }
        assert_eq!(brief.matches("\n## ").count(), TOOLS.len());
        assert!(brief.contains("The workspace is /work."));
    }

    #[test]
    fn the_brief_says_what_each_tool_opens_runs_and_teaches() {
        let brief = brief(Path::new("/work"), &[]);
        assert!(brief.contains("launch: can open .gcad .gasm .gsim."));
        assert!(brief.contains(", or a project folder."));
        assert!(brief.contains("launch: opens no files, launch it bare."));
        assert!(brief.contains("run_tool subcommands: check, measure"));
        assert!(brief.contains("run_tool: not available."));
        assert_eq!(brief.matches("skill: `").count(), 2);
        assert!(brief.contains("No skills are installed."));
    }

    #[test]
    fn a_call_is_summarised_by_what_it_does() {
        let a = json!({ "tool": "gcad", "args": ["check", "plate.gcad"] });
        let kit = super::super::Suite::default().toolkit().clone();
        assert_eq!(kit.summary("run_tool", &a).as_deref(), Some("gcad check plate.gcad"));
        let a = json!({ "tool": "gcad", "path": "a.gcad" });
        assert_eq!(kit.summary("launch", &a).as_deref(), Some("gcad a.gcad"));
        assert_eq!(kit.summary("list_tools", &json!({})), None);
    }
}
