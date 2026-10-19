use super::tools;
use crate::catalog::TOOLS;
use egui_bench::agent::skillset::Skill;
use egui_bench::agent::toolkit::Toolkit;
use egui_bench::agent::{Backend, Cx, Welcome};
use serde_json::Value;
use std::path::{Path, PathBuf};

pub struct Suite {
    kit: Toolkit,
}

impl Default for Suite {
    fn default() -> Self {
        let mut kit = Toolkit::new();
        kit.extend(tools::tools());
        Self { kit }
    }
}

impl Backend for Suite {
    fn app(&self) -> &'static str {
        "bench-suite"
    }

    fn user_agent(&self) -> String {
        concat!("bench-suite/", env!("CARGO_PKG_VERSION")).to_string()
    }

    fn brief(&self, workspace: &Path, skills: &[Skill]) -> String {
        tools::brief(workspace, skills)
    }

    fn welcome(&self) -> Welcome {
        Welcome {
            kicker: "coding agent".into(),
            title: "Design with the bench tools".into(),
            text:
                "It writes gcad parts and assemblies, agentee boards and layouts, checks them by \
                   running the tools, and opens the result in the tool's own window. It loads each \
                   tool's skill first, so it works from the manual."
                    .into(),
            placeholder:
                "Ask for a part, a board, a check. Enter sends, shift-enter breaks the line.".into(),
        }
    }

    fn toolkit(&self) -> &Toolkit {
        &self.kit
    }

    fn preferred_skills(&self) -> Vec<String> {
        TOOLS.iter().filter_map(|t| t.skill.map(String::from)).collect()
    }

    fn tool_dirs(&self, cx: &Cx) -> Vec<PathBuf> {
        let Ok(dirs) = cx.ask("dirs", Value::Null) else { return Vec::new() };
        dirs.get("dirs")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(|v| v.as_str().map(PathBuf::from)).collect())
            .unwrap_or_default()
    }
}
