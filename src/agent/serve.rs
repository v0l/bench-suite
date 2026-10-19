use crate::app::Hub;
use crate::catalog::Tool;
use egui_bench::agent::{Agent, Handled, Host, Reply};
use egui_bench::update::{Install, State};
use serde_json::{Value, json};
use std::path::PathBuf;

fn tool_index(hub: &Hub, name: &str) -> Result<usize, String> {
    let want = name.trim().to_lowercase();
    hub.slots
        .iter()
        .position(|s| s.tool.name == want || s.tool.title.to_lowercase() == want)
        .ok_or_else(|| {
            let names: Vec<&str> = hub.slots.iter().map(|s| s.tool.name).collect();
            format!("no tool called {name:?}. One of {names:?}")
        })
}

fn state_json(state: &State) -> Value {
    match state {
        State::Unchecked => json!("not checked yet"),
        State::Checking => json!("checking"),
        State::Current(r) => json!({ "latest": r.version, "update": false }),
        State::Newer(r) => json!({ "latest": r.version, "update": true }),
        State::Failed(e) => json!({ "error": e }),
    }
}

fn install_json(install: &Install) -> Value {
    match install {
        Install::Idle => json!("idle"),
        Install::Fetching { got, total } => json!({ "downloading": { "bytes": got, "of": total } }),
        Install::Unpacking => json!("installing"),
        Install::Done(m) => json!({ "done": m }),
        Install::Failed(e) => json!({ "failed": e }),
    }
}

fn opens(tool: &Tool) -> Value {
    json!({ "suffixes": tool.suffixes(), "folders": tool.takes_folders() })
}

pub fn tools_json(hub: &Hub) -> Value {
    let tools: Vec<Value> = hub
        .slots
        .iter()
        .map(|s| {
            json!({
                "name": s.tool.name,
                "title": s.tool.title,
                "about": s.tool.tagline,
                "group": s.tool.group.label(),
                "installed": s.exe.is_some(),
                "managed_by_suite": s.managed,
                "path": s.exe.as_ref().map(|p| p.display().to_string()),
                "version": s.updater.current(),
                "release": state_json(&s.updater.state()),
                "can_install": s.wants_install(hub.target),
                "install": install_json(&s.updater.install_state()),
                "opens": opens(s.tool),
                "running": s.running.len(),
                "recent": s.recent.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
            })
        })
        .collect();
    json!({
        "suite": {
            "version": env!("CARGO_PKG_VERSION"),
            "release": state_json(&hub.suite.state()),
        },
        "tools": tools,
    })
}

impl Host for Hub {
    fn agent(&mut self) -> &mut Agent<Hub> {
        &mut self.agent
    }

    fn call(&mut self, ctx: &egui::Context, name: &str, args: &Value) -> Handled<Hub> {
        match self.handle(ctx, name, args) {
            Ok(handled) => handled,
            Err(e) => Handled::Now(Err(e)),
        }
    }
}

fn tool_arg(args: &Value) -> Result<String, String> {
    args.get("tool")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .ok_or_else(|| "missing `tool`".to_string())
}

impl Hub {
    fn handle(
        &mut self,
        ctx: &egui::Context,
        name: &str,
        args: &Value,
    ) -> Result<Handled<Hub>, String> {
        match name {
            "list_tools" => Ok(Handled::Now(Ok(tools_json(self)))),
            "dirs" => {
                let mut dirs: Vec<String> = self
                    .slots
                    .iter()
                    .filter_map(|s| s.exe.as_ref()?.parent().map(|d| d.display().to_string()))
                    .collect();
                dirs.sort();
                dirs.dedup();
                Ok(Handled::Now(Ok(json!({ "dirs": dirs }))))
            }
            "path" => {
                let i = tool_index(self, &tool_arg(args)?)?;
                let path = self.slots[i].exe.as_ref().map(|p| p.display().to_string());
                Ok(Handled::Now(Ok(json!({ "path": path }))))
            }
            "check_updates" => {
                self.check(ctx);
                Ok(Handled::Later(Box::new(|hub: &mut Hub| {
                    if hub.checking() { None } else { Some(Ok(tools_json(hub))) }
                })))
            }
            "install" => {
                let i = tool_index(self, &tool_arg(args)?)?;
                let slot = &self.slots[i];
                if slot.exe.is_some() && !slot.behind() && !slot.updater.install_state().busy() {
                    return Err(format!("{} is already up to date", slot.tool.title));
                }
                self.try_install(ctx, i)?;
                Ok(Handled::Later(Box::new(move |hub: &mut Hub| hub.installed(i))))
            }
            "uninstall" => {
                let i = tool_index(self, &tool_arg(args)?)?;
                let slot = &self.slots[i];
                if !slot.managed {
                    return Err(format!("the suite did not install {}", slot.tool.title));
                }
                if self.removing.is_some() {
                    return Err("the user is already answering an uninstall question".into());
                }
                self.removing = Some(i);
                Ok(Handled::Later(Box::new(move |hub: &mut Hub| hub.removed(i))))
            }
            "launch" => {
                let i = tool_index(self, &tool_arg(args)?)?;
                let path = args.get("path").and_then(Value::as_str).map(PathBuf::from);
                if let Some(p) = &path {
                    if !p.exists() {
                        return Err(format!("{} does not exist", p.display()));
                    }
                    if !self.slots[i].tool.opens(p) {
                        let t = self.slots[i].tool;
                        return Err(format!("{} does not open that: {}", t.title, opens(t)));
                    }
                }
                let what = self.try_launch(i, path)?;
                self.say(&what, false);
                Ok(Handled::Now(Ok(json!({ "launched": what }))))
            }
            other => Err(format!("no tool called {other}")),
        }
    }

    fn installed(&self, i: usize) -> Option<Reply> {
        let slot = &self.slots[i];
        if !slot.settled || slot.updater.install_state().busy() {
            return None;
        }
        Some(match slot.updater.install_state() {
            Install::Done(m) => Ok(json!({
                "installed": slot.tool.name,
                "version": slot.updater.current(),
                "note": m,
            })),
            Install::Failed(e) => Err(e),
            other => Err(format!("install ended in {other:?}")),
        })
    }

    fn removed(&self, i: usize) -> Option<Reply> {
        if self.removing == Some(i) {
            return None;
        }
        let slot = &self.slots[i];
        Some(match slot.managed {
            false => Ok(json!({ "removed": slot.tool.name })),
            true => Err("not removed: the user kept it or the removal failed".into()),
        })
    }
}
