use crate::app::{Act, Hub, Slot};
use crate::catalog::{SUITE, Tool};
use crate::docs::{Doc, Kind, Loaded, Open, Picture, Tab};
use crate::view::{date, icon, megabytes, rule, skill_line, tidy};
use egui::{Margin, RichText, Ui, pos2, vec2};
use egui_bench::prelude::*;
use egui_bench::update::State;

const COLUMN: f32 = 900.0;
const PICTURE_MAX_H: f32 = 340.0;

#[derive(Debug, PartialEq, Eq)]
pub struct Fact {
    pub label: &'static str,
    pub value: String,
}

fn fact(label: &'static str, value: impl Into<String>) -> Fact {
    Fact { label, value: value.into() }
}

pub fn opens_line(tool: &Tool) -> String {
    match (tool.suffixes(), tool.takes_folders()) {
        ([], false) => "opens no files, start it from here".to_string(),
        (suffixes, folders) => {
            let folder = if folders { " and a project folder" } else { "" };
            format!("{}{folder}", suffixes.join("  "))
        }
    }
}

pub fn agent_line(tool: &Tool) -> Option<String> {
    if tool.cli.is_empty() {
        return Some("window only, the coding agent starts it for you".to_string());
    }
    Some(format!("the coding agent can run: {}", tool.cli.join(", ")))
}

pub fn facts(slot: &Slot) -> Vec<Fact> {
    let tool = slot.tool;
    let state = slot.updater.state();
    let mut out = Vec::new();
    match (&slot.exe, slot.updater.current()) {
        (Some(_), Some(v)) => out.push(fact("installed", v)),
        (Some(_), None) => out.push(fact("installed", "unknown version")),
        (None, _) => out.push(fact("installed", "not installed")),
    }
    if let Some(r) = state.release() {
        let when = date(&r.published).map(|d| format!(", released {d}")).unwrap_or_default();
        let behind = matches!(state, State::Newer(_)) && slot.exe.is_some();
        let mark = if behind { " (newer than yours)" } else { "" };
        out.push(fact("latest", format!("{}{when}{mark}", r.version)));
    }
    if let Some((_, asset)) = slot.installable(crate::catalog::Target::host()) {
        out.push(fact("download", megabytes(asset.bytes)));
    }
    if let Some(exe) = &slot.exe {
        let how =
            if slot.managed { "kept up to date by the suite" } else { "found on this machine" };
        out.push(fact("where", format!("{} ({how})", tidy(exe))));
    }
    out.push(fact("opens", opens_line(tool)));
    if let Some(line) = agent_line(tool) {
        out.push(fact("agent", line));
    }
    if tool.skill.is_some() {
        let line = match (&slot.skills, slot.exe.is_some(), slot.managed) {
            (Ok(placed), _, _) if !placed.is_empty() => skill_line(tool, placed),
            (_, true, false) => {
                format!(
                    "the {} skill is installed when the suite installs or updates it",
                    tool.name
                )
            }
            _ => format!("the {} skill comes with it once installed", tool.name),
        };
        out.push(fact("skill", line));
    }
    out
}

impl Hub {
    pub fn page_view(&mut self, ui: &mut Ui, acts: &mut Vec<Act>) {
        let Some(open) = self.reader.open.clone() else { return };
        let ctx = ui.ctx().clone();
        let is_suite = open.repo == SUITE.repo;
        let slot_at = self.slots.iter().position(|s| s.tool.repo == open.repo);
        let title = slot_at.map(|i| self.slots[i].tool.title).unwrap_or(SUITE.title);
        let tagline = slot_at.map(|i| self.slots[i].tool.tagline).unwrap_or("");
        let repo_page = format!("https://github.com/{}", open.repo);

        let mut close = false;
        let mut switch = None;
        let area = ui.max_rect();
        let width = (area.width() - 32.0).clamp(120.0, COLUMN);
        let left = area.center().x - width / 2.0;
        let page =
            egui::Rect::from_min_max(pos2(left, area.top()), pos2(left + width, area.bottom()));
        ui.scope_builder(egui::UiBuilder::new().max_rect(page), |ui| {
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if ui.add(Key::new("back").outline(VALUE)).clicked() {
                    close = true;
                }
                ui.add_space(12.0);
                if let Some(i) = slot_at {
                    ui.add(
                        egui::Image::new(icon(self.slots[i].tool))
                            .fit_to_exact_size(vec2(34.0, 34.0)),
                    );
                } else {
                    ui.add(egui::Image::new(icon(&SUITE)).fit_to_exact_size(vec2(34.0, 34.0)));
                }
                ui.add_space(6.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new(title).font(legend_font(20.0)).color(VALUE));
                    if !tagline.is_empty() {
                        Line::new().note(tagline).size(12.0).show(ui);
                    }
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add(Key::new("source").outline(VALUE)).on_hover_text(&repo_page).clicked()
                    {
                        acts.push(Act::Url(repo_page.clone()));
                    }
                    ui.add_space(8.0);
                    if let Some(i) = slot_at {
                        self.page_primary(ui, i, acts);
                    }
                });
            });
            ui.add_space(10.0);
            let mut tab = open.tab;
            let options: Vec<(Tab, &str, Option<usize>)> =
                [Tab::Overview, Tab::Readme, Tab::Changelog]
                    .iter()
                    .filter(|t| !(is_suite && **t == Tab::Overview))
                    .map(|t| (*t, t.label(), None))
                    .collect();
            if segments(ui, &mut tab, &options) {
                switch = Some(tab);
            }
            ui.add_space(8.0);
            rule(ui, ui.max_rect().left_top(), ui.max_rect().right_top());
            ui.add_space(8.0);
            match open.tab {
                Tab::Overview => {
                    if let Some(i) = slot_at {
                        self.overview(ui, i, &ctx, acts);
                    }
                }
                Tab::Readme | Tab::Changelog => {
                    let kind = open.tab.kind().unwrap_or(Kind::Readme);
                    self.document(ui, &Doc { repo: open.repo.clone(), kind });
                }
            }
        });
        self.reader.cache.sweep();
        if let Some(tab) = switch {
            self.reader.open = Some(Open { repo: open.repo.clone(), tab });
        }
        if close {
            self.reader.open = None;
        }
    }

    fn page_primary(&self, ui: &mut Ui, i: usize, acts: &mut Vec<Act>) {
        let slot = &self.slots[i];
        let installable = slot.installable(self.target).is_some();
        match (&slot.exe, slot.wants_install(self.target)) {
            (Some(_), true) if installable => {
                if ui.add(Key::new("update").solid(READOUT)).clicked() {
                    acts.push(Act::Install(i));
                }
                ui.add_space(8.0);
                self.launch_key(ui, i, acts);
            }
            (Some(_), _) => self.launch_key(ui, i, acts),
            (None, _) => {
                if ui.add(Key::new("install").solid(READOUT).enabled(installable)).clicked() {
                    acts.push(Act::Install(i));
                }
            }
        }
    }

    fn launch_key(&self, ui: &mut Ui, i: usize, acts: &mut Vec<Act>) {
        let tool = self.slots[i].tool;
        if tool.takes_folders() {
            if ui.add(Key::new("open project").solid(VALUE)).clicked() {
                acts.push(Act::Pick(i));
            }
        } else if ui.add(Key::new("launch").solid(VALUE)).clicked() {
            acts.push(Act::Launch(i, None));
        }
    }

    fn overview(&mut self, ui: &mut Ui, i: usize, ctx: &egui::Context, acts: &mut Vec<Act>) {
        let tool = self.slots[i].tool;
        let facts = facts(&self.slots[i]);
        let picture = self.reader.pictures.get(tool.repo, ctx);
        let recent: Vec<std::path::PathBuf> = self.slots[i].recent.clone();
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.set_width(ui.available_width());
            Line::new().note(tool.blurb).size(13.5).tint(VALUE).wrapped(ui);
            ui.add_space(12.0);
            section(ui, "what it can do", "", |ui| {
                for line in tool.features {
                    ui.horizontal_top(|ui| {
                        let (rect, _) =
                            ui.allocate_exact_size(vec2(14.0, 18.0), egui::Sense::hover());
                        ui.painter().circle_filled(rect.center(), 2.5, READOUT);
                        ui.add(
                            egui::Label::new(RichText::new(*line).size(13.0).color(VALUE)).wrap(),
                        );
                    });
                }
            });
            ui.add_space(12.0);
            match picture {
                Picture::Bytes { uri, bytes } => {
                    egui::Frame::NONE
                        .stroke(egui::Stroke::new(1.0, ETCH))
                        .corner_radius(2)
                        .inner_margin(Margin::same(1))
                        .show(ui, |ui| {
                            ui.add(
                                egui::Image::from_bytes(uri, bytes.to_vec())
                                    .max_size(vec2(ui.available_width(), PICTURE_MAX_H))
                                    .corner_radius(2),
                            );
                        });
                    ui.add_space(12.0);
                }
                Picture::Looking => {
                    ui.horizontal(|ui| {
                        ui.add(egui::Spinner::new().size(12.0));
                        ui.add_space(6.0);
                        Line::new().legend("looking for a screenshot").size(11.0).show(ui);
                    });
                    ui.add_space(12.0);
                }
                Picture::None | Picture::Failed(_) => {}
            }
            section(ui, "details", "", |ui| {
                for f in &facts {
                    reading(ui, f.label, f.value.clone());
                }
            });
            if !recent.is_empty() {
                ui.add_space(12.0);
                section(ui, "recent", "click to open again", |ui| {
                    for path in &recent {
                        let there = path.exists();
                        let text = RichText::new(crate::app::file_label(path))
                            .size(13.0)
                            .color(if there { VALUE } else { LEGEND });
                        if ui
                            .add_enabled(there, egui::Button::new(text))
                            .on_hover_text(tidy(path))
                            .clicked()
                        {
                            acts.push(Act::Launch(i, Some(path.clone())));
                        }
                    }
                });
            }
            ui.add_space(24.0);
        });
    }

    fn document(&mut self, ui: &mut Ui, doc: &Doc) {
        let ctx = ui.ctx().clone();
        match self.reader.library.get(doc, &ctx) {
            Loaded::Loading => {
                ui.horizontal(|ui| {
                    ui.add(egui::Spinner::new().size(12.0));
                    ui.add_space(6.0);
                    Line::new().legend("loading").size(11.0).show(ui);
                });
            }
            Loaded::Failed(e) => {
                Line::new().value(e).tint(FAULT).wrapped(ui);
                ui.add_space(8.0);
                if ui.add(Key::new("try again").outline(VALUE)).clicked() {
                    self.reader.library.forget(doc);
                }
            }
            Loaded::Ready(text) => {
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(Key::new("open in browser").outline(READOUT)).clicked() {
                            ctx.open_url(egui::OpenUrl::new_tab(doc.page()));
                        }
                    });
                });
                ui.add_space(4.0);
                let base = doc.base();
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    egui_bench::markdown::show_at(
                        ui,
                        &mut self.reader.cache,
                        &text,
                        0,
                        Some(&base),
                    );
                    ui.add_space(24.0);
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::TOOLS;

    fn tool(name: &str) -> &'static Tool {
        TOOLS.iter().find(|t| t.name == name).unwrap()
    }

    #[test]
    fn what_a_tool_opens_is_said_plainly() {
        assert_eq!(opens_line(tool("gcad")), ".gcad  .gasm  .gsim");
        assert!(opens_line(tool("agentee")).ends_with("and a project folder"));
        assert_eq!(opens_line(tool("cycler")), "opens no files, start it from here");
    }

    #[test]
    fn a_tool_that_is_not_installed_still_gets_a_page_of_facts() {
        let hub_tool = tool("battery-control");
        assert!(hub_tool.features.len() >= 4);
        assert_eq!(opens_line(hub_tool), "opens no files, start it from here");
        assert!(agent_line(hub_tool).unwrap().contains("window only"));
    }

    #[test]
    fn a_tool_with_a_command_line_names_what_the_agent_can_run_and_one_without_says_so() {
        assert!(agent_line(tool("gcad")).unwrap().contains("check, measure"));
        assert!(agent_line(tool("printfarm")).unwrap().contains("window only"));
    }
}
