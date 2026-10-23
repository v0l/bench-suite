use crate::app::{Act, Filter, Hub, Slot, file_label};
use crate::catalog::{Group, SUITE, Target, Tool};
use crate::install;
use crate::skills::{self, How, Placed};
use egui::text::{LayoutJob, TextFormat};
use egui::{
    Color32, CornerRadius, FontId, Id, ImageSource, Pos2, Rect, Response, Sense, Stroke,
    StrokeKind, Ui, Vec2, pos2, vec2,
};
use egui_bench::prelude::*;
use egui_bench::update::{Install, State};
use std::path::Path;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Tools,
    Agent,
}

const PAGE: f32 = 20.0;
const GAP: f32 = 12.0;
const CARD_MIN_W: f32 = 320.0;
const CARD_H: f32 = 172.0;
const BAY_GAP: f32 = 20.0;
const HEAD_H: f32 = 28.0;
const PAD: f32 = 16.0;
const ICON: f32 = 48.0;
const KEY_H: f32 = 30.0;
const R: CornerRadius = CornerRadius::same(2);

pub fn icon(tool: &Tool) -> ImageSource<'static> {
    match tool.name {
        "gcad" => egui::include_image!("../assets/icons/gcad.svg"),
        "agentee" => egui::include_image!("../assets/icons/agentee.svg"),
        "antenna-toolbox" => egui::include_image!("../assets/icons/antenna-toolbox.svg"),
        "printfarm" => egui::include_image!("../assets/icons/printfarm.svg"),
        "thermview" => egui::include_image!("../assets/icons/thermview.svg"),
        "battery-control" => egui::include_image!("../assets/icons/battery-control.svg"),
        "cycler" => egui::include_image!("../assets/icons/cycler.svg"),
        "waveshark" => egui::include_image!("../assets/icons/waveshark.svg"),
        "radio_tool" => egui::include_image!("../assets/icons/radio_tool.svg"),
        "foreman" => egui::include_image!("../assets/icons/foreman.svg"),
        "coldbench" => egui::include_image!("../assets/icons/coldbench.svg"),
        _ => egui::include_image!("../assets/icons/bench-suite.svg"),
    }
}

impl eframe::App for Hub {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.reap(&ctx);
        self.agent.poll();
        self.dropped(&ctx);
        let mut acts = Vec::new();
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::F))
            && let Some(id) = ctx.data(|d| d.get_temp::<Id>(search_id()))
        {
            ctx.memory_mut(|m| m.request_focus(id));
        }
        egui::Panel::top("header")
            .frame(egui::Frame::NONE.fill(CHASSIS).inner_margin(egui::Margin::symmetric(20, 12)))
            .show(ui, |ui| self.header(ui, &mut acts));
        if self.suite_banner_wanted() {
            egui::Panel::top("suite")
                .frame(egui::Frame::NONE.fill(PANEL).inner_margin(egui::Margin::symmetric(20, 8)))
                .show(ui, |ui| self.suite_banner(ui, &mut acts));
        }
        egui::Panel::bottom("footer")
            .frame(egui::Frame::NONE.fill(CHASSIS).inner_margin(egui::Margin::symmetric(20, 6)))
            .show(ui, |ui| self.footer(ui));
        egui::CentralPanel::no_frame().show(ui, |ui| {
            let r = ui.max_rect();
            ui.painter().rect_filled(r, 0.0, CHASSIS);
            rule(ui, r.left_top(), r.right_top());
            if self.reader.open.is_some() {
                self.page_view(ui, &mut acts);
                return;
            }
            if self.agent.open {
                self.agent.show(ui);
                return;
            }
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                egui::Frame::NONE
                    .inner_margin(egui::Margin::symmetric(PAGE as i8, 16))
                    .show(ui, |ui| self.body(ui, &mut acts));
            });
        });
        self.confirm_removal(&ctx, &mut acts);
        self.apply(&ctx, acts);
        egui_bench::agent::serve(self, &ctx);
        self.browse(&ctx);
        self.drop_overlay(&ctx);
        let live = self.slots.iter().any(|s| !s.running.is_empty() || s.updater.busy())
            || self.suite.busy()
            || self.agent.live()
            || self.agent.needs_you();
        if live {
            ctx.request_repaint_after(Duration::from_millis(250));
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.save_recent(storage);
    }
}

impl Hub {
    fn header(&mut self, ui: &mut Ui, acts: &mut Vec<Act>) {
        let checking = self.checking();
        ui.horizontal(|ui| {
            ui.add(egui::Image::new(icon(&SUITE)).fit_to_exact_size(Vec2::splat(26.0)));
            ui.add_space(4.0);
            let mut job = LayoutJob::default();
            job.append("BENCH SUITE", 0.0, tracked(legend_font(16.0), VALUE, 2.0));
            job.append(env!("CARGO_PKG_VERSION"), 10.0, face(figure(12.0), LEGEND));
            ui.label(job);
            ui.add_space(28.0);
            self.page_tabs(ui);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let label = if checking { "checking" } else { "check for updates" };
                let r = ui
                    .add(Key::new(label).outline(VALUE).enabled(!checking))
                    .on_hover_text("ask GitHub for the newest release of every tool");
                if r.clicked() {
                    acts.push(Act::Check);
                }
                ui.add_space(8.0);
                let room = (ui.available_width() - 16.0).min(240.0);
                if room < 120.0 || self.agent.open {
                    return;
                }
                ui.scope(|ui| {
                    ui.set_width(room);
                    let r = egui_bench::form::field(ui, &mut self.search, "search tools");
                    let id = r.id;
                    ui.ctx().data_mut(|d| d.insert_temp(search_id(), id));
                    if r.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        self.search.clear();
                    }
                });
            });
        });
    }

    fn page_tabs(&mut self, ui: &mut Ui) {
        let mut page = if self.agent.open { Page::Agent } else { Page::Tools };
        let before = page;
        let agent = match (self.agent.needs_you(), self.agent.chat.busy()) {
            (true, _) => "agent needs you",
            (false, true) => "agent working",
            (false, false) => "agent",
        };
        segments(ui, &mut page, &[(Page::Tools, "tools", None), (Page::Agent, agent, None)]);
        if page != before {
            self.reader.open = None;
        }
        self.agent.open = page == Page::Agent;
    }

    fn suite_banner_wanted(&self) -> bool {
        matches!(self.suite.state(), State::Newer(_)) || self.suite.install_state() != Install::Idle
    }

    fn suite_banner(&mut self, ui: &mut Ui, acts: &mut Vec<Act>) {
        let r = ui.max_rect();
        ui.painter().rect_filled(
            Rect::from_min_max(
                r.left_top() - vec2(20.0, 8.0),
                pos2(r.left() - 17.0, r.bottom() + 8.0),
            ),
            0.0,
            READOUT,
        );
        let latest = self.suite.state().release().map(|r| r.version.clone()).unwrap_or_default();
        ui.horizontal(|ui| {
            let install = self.suite.install_state();
            let line = match &install {
                Install::Done(_) => Line::new()
                    .legend("bench suite")
                    .set(latest.clone())
                    .note("is installed. Restart to use it."),
                Install::Failed(e) => Line::new()
                    .legend("bench suite")
                    .note(format!("could not update: {e}"))
                    .tint(FAULT),
                Install::Fetching { .. } => {
                    Line::new().legend("bench suite").set(latest.clone()).note("is on its way")
                }
                Install::Unpacking => {
                    Line::new().legend("bench suite").set(latest.clone()).note("installing")
                }
                Install::Idle => Line::new()
                    .legend("bench suite")
                    .set(latest.clone())
                    .note(format!("is out. You have {}.", env!("CARGO_PKG_VERSION"))),
            };
            line.show(ui);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| match install {
                Install::Done(_) => {
                    if ui.add(Key::new("restart").solid(READOUT)).clicked() {
                        acts.push(Act::Restart);
                    }
                }
                Install::Idle | Install::Failed(_) => {
                    let ready = self.target.is_some();
                    if ui.add(Key::new("update the suite").solid(READOUT).enabled(ready)).clicked()
                    {
                        acts.push(Act::UpdateSelf);
                    }
                    ui.add_space(8.0);
                    if ui
                        .add(Key::new("what is new").outline(VALUE))
                        .on_hover_text("read the changelog before updating")
                        .clicked()
                    {
                        acts.push(Act::SuitePage(crate::docs::Tab::Changelog));
                    }
                }
                busy_state => {
                    if let Some((done, label)) = progress(&busy_state) {
                        let (rect, _) = ui.allocate_exact_size(vec2(260.0, KEY_H), Sense::hover());
                        busy(ui, rect, done, &label, READOUT);
                    }
                }
            });
        });
    }

    fn footer(&mut self, ui: &mut Ui) {
        let r = ui.max_rect();
        rule(ui, r.left_top() - vec2(20.0, 6.0), r.right_top() + vec2(20.0, -6.0));
        ui.horizontal(|ui| {
            match self.status.clone() {
                Some((msg, fault)) => {
                    let (tint, dot) = if fault { (FAULT, FAULT) } else { (VALUE, OK) };
                    dot_ui(ui, dot);
                    Line::new().note(msg).tint(tint).show(ui);
                    if ui.add(Key::new("dismiss").outline(VALUE)).clicked() {
                        self.status = None;
                    }
                }
                None if !self.checking() && self.check_failure().is_some() => {
                    dot_ui(ui, FAULT);
                    let e = self.check_failure().unwrap_or_default();
                    Line::new()
                        .note(format!("could not check for updates: {e}"))
                        .tint(FAULT)
                        .show(ui);
                }
                None if self.slots.iter().any(|s| s.updater.install_state().busy()) => {
                    let names: Vec<&str> = self
                        .slots
                        .iter()
                        .filter(|s| s.updater.install_state().busy())
                        .map(|s| s.tool.title)
                        .collect();
                    dot_ui(ui, READOUT);
                    Line::new()
                        .note(format!("installing {}", names.join(", ")))
                        .tint(VALUE)
                        .show(ui);
                }
                None => {
                    let text = match (self.checking(), self.checked) {
                        (true, _) => "asking GitHub for the newest releases".to_string(),
                        (false, Some(at)) => format!("checked for updates {}", ago(at)),
                        (false, None) => "not checked for updates yet".to_string(),
                    };
                    Line::new().note(text).show(ui);
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let target = self.target.map(Target::label).unwrap_or("no published build");
                Line::new()
                    .note(format!("installing for {target}"))
                    .show(ui)
                    .on_hover_text(format!("tools are kept in {}", tidy(&install::root())));
            });
        });
    }

    fn body(&mut self, ui: &mut Ui, acts: &mut Vec<Act>) {
        self.filter_row(ui, acts);
        ui.add_space(20.0);
        let visible: Vec<usize> =
            (0..self.slots.len()).filter(|&i| self.matches(&self.slots[i])).collect();
        if visible.is_empty() {
            return self.empty(ui, acts);
        }
        let whole = self.filter == Filter::All && self.search.trim().is_empty();
        let width = ui.available_width();
        let bays = Group::ALL.len() as f32;
        if whole && width >= bays * CARD_MIN_W + (bays - 1.0) * BAY_GAP {
            self.bays(ui, acts);
        } else if whole {
            for group in Group::ALL {
                let members: Vec<usize> = visible
                    .iter()
                    .copied()
                    .filter(|&i| self.slots[i].tool.group == group)
                    .collect();
                heading(ui, group.label());
                ui.add_space(10.0);
                self.grid(ui, &members, acts);
                ui.add_space(14.0);
            }
        } else {
            self.grid(ui, &visible, acts);
        }
    }

    fn bays(&self, ui: &mut Ui, acts: &mut Vec<Act>) {
        let width = ui.available_width();
        let n = Group::ALL.len();
        let bay_w = (width - BAY_GAP * (n as f32 - 1.0)) / n as f32;
        let tallest = Group::ALL
            .iter()
            .map(|g| self.slots.iter().filter(|s| s.tool.group == *g).count())
            .max()
            .unwrap_or(0) as f32;
        let height = HEAD_H + tallest * (CARD_H + GAP);
        let (area, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
        for (b, group) in Group::ALL.iter().enumerate() {
            let left = area.left() + b as f32 * (bay_w + BAY_GAP);
            let mut child = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(Rect::from_min_size(pos2(left, area.top()), vec2(bay_w, height))),
            );
            heading(&mut child, group.label());
            let mut top = area.top() + HEAD_H;
            for i in (0..self.slots.len()).filter(|&i| self.slots[i].tool.group == *group) {
                let rect = Rect::from_min_size(pos2(left, top), vec2(bay_w, CARD_H));
                if ui.is_rect_visible(rect) {
                    self.card(ui, rect, i, acts);
                }
                top += CARD_H + GAP;
            }
        }
    }

    fn filter_row(&mut self, ui: &mut Ui, acts: &mut Vec<Act>) {
        let pending = self.pending();
        let all = Key::new(format!("install all ({pending})")).solid(READOUT);
        let all_w = all.width(ui);
        let mut below = false;
        ui.horizontal(|ui| {
            let options = [
                (Filter::All, "all", Some(self.count(Filter::All))),
                (Filter::Installed, "installed", Some(self.count(Filter::Installed))),
                (Filter::Updates, "updates", Some(self.count(Filter::Updates))),
                (Filter::Missing, "not installed", Some(self.count(Filter::Missing))),
            ];
            segments(ui, &mut self.filter, &options);
            if pending == 0 {
                return;
            }
            if ui.available_width() < all_w + 16.0 {
                below = true;
                return;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                install_all(ui, all, acts);
            });
        });
        if below {
            ui.add_space(8.0);
            install_all(ui, Key::new(format!("install all ({pending})")).solid(READOUT), acts);
        }
    }

    fn grid(&self, ui: &mut Ui, members: &[usize], acts: &mut Vec<Act>) {
        let width = ui.available_width();
        let cols = ((width + GAP) / (CARD_MIN_W + GAP)).floor().max(1.0) as usize;
        let card_w = (width - GAP * (cols as f32 - 1.0)) / cols as f32;
        for row in members.chunks(cols) {
            let (strip, _) = ui.allocate_exact_size(vec2(width, CARD_H), Sense::hover());
            for (c, &i) in row.iter().enumerate() {
                let x = strip.left() + c as f32 * (card_w + GAP);
                let rect = Rect::from_min_size(pos2(x, strip.top()), vec2(card_w, CARD_H));
                if ui.is_rect_visible(rect) {
                    self.card(ui, rect, i, acts);
                }
            }
            ui.add_space(GAP);
        }
    }

    fn card(&self, ui: &mut Ui, rect: Rect, i: usize, acts: &mut Vec<Act>) {
        let slot = &self.slots[i];
        let tool = slot.tool;
        let state = slot.updater.state();
        let install = slot.updater.install_state();
        let current = slot.updater.current();
        let release = state.release().cloned();
        let behind = slot.behind();
        let running = slot.running.len();
        let installable = slot.installable(self.target);

        let hovered = ui.rect_contains_pointer(rect);
        let open_page = ui.interact(rect, Id::new(("card", i)), Sense::click());
        if hovered {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if open_page.clicked() {
            acts.push(Act::Page(i, crate::docs::Tab::Overview));
        }
        let p = ui.painter().clone();
        p.rect_filled(rect, R, PANEL);
        p.rect_stroke(
            rect,
            R,
            Stroke::new(1.0, if hovered { BAND } else { ETCH }),
            StrokeKind::Inside,
        );
        if running > 0 || behind {
            let rail = if running > 0 { OK } else { READOUT };
            p.rect_filled(
                Rect::from_min_size(rect.left_top(), vec2(3.0, rect.height())),
                CornerRadius { nw: 2, sw: 2, ne: 0, se: 0 },
                rail,
            );
        }

        let icon_rect = Rect::from_min_size(rect.left_top() + vec2(PAD, PAD), Vec2::splat(ICON));
        egui::Image::new(icon(tool)).paint_at(ui, icon_rect);
        if running > 0 {
            let c = icon_rect.right_bottom() - vec2(2.0, 2.0);
            p.circle_filled(c, 6.0, PANEL);
            p.circle_filled(c, 4.0, OK);
        }

        let text_x = icon_rect.right() + 14.0;
        let mut name_right = rect.right() - PAD;
        if running > 0 {
            let text = if running > 1 { format!("running {running}") } else { "running".into() };
            let corner = Rect::from_min_max(
                pos2(text_x, rect.top() + PAD),
                pos2(rect.right() - PAD, rect.top() + PAD + 22.0),
            );
            let mut child = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(corner)
                    .layout(egui::Layout::right_to_left(egui::Align::Min)),
            );
            name_right = chip(&mut child, &text, OK, true).rect.left() - 8.0;
        }
        let name = elide(ui, tool.title, face(legend_font(20.0), VALUE), name_right - text_x);
        p.galley(pos2(text_x, icon_rect.top() + 1.0), name, VALUE);
        let tag_w = rect.right() - PAD - text_x;
        let tagline = elide(ui, tool.tagline, face(FontId::proportional(13.0), LEGEND), tag_w);
        p.galley(pos2(text_x, icon_rect.top() + 28.0), tagline, LEGEND);

        let inner_w = rect.width() - 2.0 * PAD;
        let row1 = pos2(rect.left() + PAD, icon_rect.bottom() + 16.0);
        let version = version_line(slot, current.as_deref(), &state, release.as_ref());
        let version = elide_job(ui, version, inner_w);
        p.galley(row1, version, VALUE);

        let row2 = Rect::from_min_size(row1 + vec2(0.0, 22.0), vec2(inner_w, 16.0));
        self.detail(ui, row2, slot, &install, &state, installable.as_ref().map(|(_, a)| a.bytes));

        let keys_top = rect.bottom() - PAD - KEY_H;
        let menu_rect =
            Rect::from_min_size(pos2(rect.right() - PAD - KEY_H, keys_top), Vec2::splat(KEY_H));
        let row_rect = Rect::from_min_max(
            pos2(rect.left() + PAD, keys_top),
            pos2(menu_rect.left() - 8.0, keys_top + KEY_H),
        );
        if let Some((done, label)) = progress(&install) {
            busy(ui, row_rect, done, &label, READOUT);
        } else {
            self.keys(
                ui,
                row_rect,
                i,
                &state,
                release.as_ref(),
                current.as_deref(),
                installable.is_some(),
                acts,
            );
        }
        let menu = Key::more().id_salt(("menu", i)).paint(ui, menu_rect).on_hover_text("more");
        self.card_menu(&menu, i, release.as_ref().map(|r| r.page.clone()), acts);
    }

    #[allow(clippy::too_many_arguments)]
    fn keys(
        &self,
        ui: &mut Ui,
        row: Rect,
        i: usize,
        state: &State,
        release: Option<&egui_bench::update::Release>,
        current: Option<&str>,
        installable: bool,
        acts: &mut Vec<Act>,
    ) {
        let slot = &self.slots[i];
        let secondary = match (&slot.exe, release) {
            (Some(_), Some(r)) if matches!(state, State::Newer(_)) && installable => {
                let label = if slot.managed && current.is_some() { "update" } else { "install" };
                let hover = if slot.managed {
                    format!("download and install {}", r.version)
                } else {
                    format!(
                        "install {} into the suite's own folder, used from then on instead of the copy it found",
                        r.version
                    )
                };
                Some((label, hover))
            }
            _ => None,
        };
        let second_w = secondary.as_ref().map(|(l, _)| Key::new(*l).width(ui) + 8.0).unwrap_or(0.0);
        let primary_rect =
            Rect::from_min_max(row.left_top(), pos2(row.right() - second_w, row.bottom()));
        match &slot.exe {
            Some(_) if slot.tool.takes_folders() => {
                let resp = Key::new("open project")
                    .solid(VALUE)
                    .id_salt(("launch", i))
                    .paint(ui, primary_rect)
                    .on_hover_text(format!("pick a project folder to open in {}", slot.tool.title));
                if resp.clicked() {
                    acts.push(Act::Pick(i));
                }
            }
            Some(exe) => {
                let resp = Key::new("launch")
                    .solid(VALUE)
                    .id_salt(("launch", i))
                    .paint(ui, primary_rect)
                    .on_hover_text(format!("start {}", exe.display()));
                if resp.clicked() {
                    acts.push(Act::Launch(i, None));
                }
            }
            None => {
                if Key::new("install")
                    .solid(READOUT)
                    .enabled(installable)
                    .id_salt(("install", i))
                    .paint(ui, primary_rect)
                    .clicked()
                {
                    acts.push(Act::Install(i));
                }
            }
        }
        if let Some((label, hover)) = secondary {
            let r = Rect::from_min_max(
                pos2(row.right() - second_w + 8.0, row.top()),
                row.right_bottom(),
            );
            if Key::new(label)
                .outline(READOUT)
                .id_salt(("second", i))
                .paint(ui, r)
                .on_hover_text(hover)
                .clicked()
            {
                acts.push(Act::Install(i));
            }
        }
    }

    fn detail(
        &self,
        ui: &mut Ui,
        row: Rect,
        slot: &Slot,
        install: &Install,
        state: &State,
        bytes: Option<u64>,
    ) {
        let p = ui.painter().clone();
        let note = |text: String, tint: Color32| {
            let g = elide(ui, &text, face(FontId::proportional(12.5), tint), row.width());
            p.galley(row.left_top(), g, tint);
        };
        match install {
            Install::Fetching { .. } | Install::Unpacking => {
                let to = state.release().map(|r| r.version.clone()).unwrap_or_default();
                let what = match (&slot.exe, slot.managed) {
                    (Some(_), true) => format!("updating to {to}"),
                    _ => format!("installing {to}"),
                };
                return note(what, LEGEND);
            }
            Install::Failed(e) => return note(e.clone(), FAULT),
            Install::Done(msg) if slot.exe.is_none() => return note(msg.clone(), OK),
            _ => {}
        }
        if let (State::Failed(e), None) = (state, &slot.exe) {
            note("waiting on GitHub for the latest release".into(), LEGEND);
            ui.interact(row, Id::new(("why", slot.tool.name)), Sense::hover()).on_hover_text(e);
            return;
        }
        if slot.managed && !slot.behind() && slot.tool.skill.is_some() {
            match &slot.skills {
                Err(e) => return note(e.clone(), FAULT),
                Ok(placed) if !placed.is_empty() => {
                    note(skill_line(slot.tool, placed), LEGEND);
                    ui.interact(row, Id::new(("skill", slot.tool.name)), Sense::hover())
                        .on_hover_text(skill_hover(placed));
                    return;
                }
                Ok(_) => {}
            }
        }
        let target = self.target.map(Target::label).unwrap_or("this machine");
        match (&slot.exe, state.release(), bytes) {
            (Some(exe), release, bytes) => {
                let released = release.and_then(|r| date(&r.published));
                let text = match (slot.managed, slot.behind(), release, released, bytes) {
                    (true, true, Some(r), Some(d), Some(b)) => {
                        format!("{} came out {d}, a {} download", r.version, megabytes(b))
                    }
                    (true, false, _, Some(d), _) => format!("released {d}"),
                    (true, _, _, _, _) => "kept up to date by the suite".to_string(),
                    (false, _, _, _, _) => format!("found at {}", tidy(exe)),
                };
                note(text, LEGEND);
                ui.interact(row, Id::new(("where", slot.tool.name)), Sense::hover())
                    .on_hover_text(exe.display().to_string());
            }
            (None, Some(r), Some(b)) => {
                let when =
                    date(&r.published).map(|d| format!("released {d}, ")).unwrap_or_default();
                note(format!("{when}{} download", megabytes(b)), LEGEND)
            }
            (None, Some(r), None) => note(format!("{} has no build for {target}", r.tag), LEGEND),
            (None, None, _) => note("looking for the latest release".into(), LEGEND),
        }
    }

    fn card_menu(&self, button: &Response, i: usize, page: Option<String>, acts: &mut Vec<Act>) {
        let slot = &self.slots[i];
        let tool = slot.tool;
        egui::Popup::menu(button).width(240.0).show(|ui| {
            ui.spacing_mut().button_padding = vec2(10.0, 6.0);
            if slot.exe.is_some() && !tool.suffixes().is_empty() {
                let label =
                    if tool.takes_folders() { "Open a project…" } else { "Open a file…" };
                if ui.add(Key::new(label).outline(VALUE).min_width(ui.available_width())).clicked()
                {
                    acts.push(Act::Pick(i));
                }
                if !slot.recent.is_empty() {
                    ui.add_space(4.0);
                    ui.label(legend("recent"));
                    for path in &slot.recent {
                        let there = path.exists();
                        let text = file_label(path);
                        if ui
                            .add(
                                Key::new(text)
                                    .outline(if there { VALUE } else { LEGEND })
                                    .enabled(there)
                                    .min_width(ui.available_width()),
                            )
                            .on_hover_text(tidy(path))
                            .clicked()
                        {
                            acts.push(Act::Launch(i, Some(path.clone())));
                        }
                    }
                    if ui
                        .add(
                            Key::new("Forget recent files")
                                .outline(VALUE)
                                .min_width(ui.available_width()),
                        )
                        .clicked()
                    {
                        acts.push(Act::Forget(i));
                    }
                }
            }
            if ui
                .add(Key::new("Open its page").outline(VALUE).min_width(ui.available_width()))
                .clicked()
            {
                acts.push(Act::Page(i, crate::docs::Tab::Overview));
            }
            if ui
                .add(Key::new("Read the README").outline(VALUE).min_width(ui.available_width()))
                .clicked()
            {
                acts.push(Act::Page(i, crate::docs::Tab::Readme));
            }
            if ui
                .add(Key::new("Read the changelog").outline(VALUE).min_width(ui.available_width()))
                .clicked()
            {
                acts.push(Act::Page(i, crate::docs::Tab::Changelog));
            }
            if let Some(page) = page.filter(|p| !p.is_empty())
                && ui
                    .add(
                        Key::new("Release page on GitHub")
                            .outline(VALUE)
                            .min_width(ui.available_width()),
                    )
                    .clicked()
            {
                acts.push(Act::Url(page));
            }
            if ui
                .add(Key::new("Source on GitHub").outline(VALUE).min_width(ui.available_width()))
                .clicked()
            {
                acts.push(Act::Url(format!("https://github.com/{}", tool.repo)));
            }
            if slot.exe.is_some()
                && ui
                    .add(Key::new("Show in folder").outline(VALUE).min_width(ui.available_width()))
                    .clicked()
            {
                acts.push(Act::Reveal(i));
            }
            if let Ok(placed) = &slot.skills
                && let Some(shared) = placed.iter().find(|p| p.agent.is_none())
                && ui
                    .add(
                        Key::new("Show agent skill").outline(VALUE).min_width(ui.available_width()),
                    )
                    .on_hover_text(tidy(&shared.path))
                    .clicked()
            {
                acts.push(Act::RevealPath(shared.path.clone()));
            }
            if slot.managed
                && ui
                    .add(Key::new("Uninstall").outline(FAULT).min_width(ui.available_width()))
                    .on_hover_text(format!("delete {}", tidy(&install::home(tool))))
                    .clicked()
            {
                acts.push(Act::Uninstall(i));
            }
        });
    }

    fn empty(&self, ui: &mut Ui, acts: &mut Vec<Act>) {
        let (title, body) = if !self.search.trim().is_empty() {
            (
                format!("No tool matches \"{}\"", self.search.trim()),
                "Try a word from what it does, like CAD or radio.".to_string(),
            )
        } else {
            match self.filter {
                Filter::Updates => (
                    "Everything is up to date".into(),
                    "Installed tools are on their latest release.".into(),
                ),
                Filter::Installed => (
                    "Nothing is installed yet".into(),
                    "Install a tool from the full list, or all of them at once.".into(),
                ),
                Filter::Missing => {
                    ("Every tool is installed".into(), "Launch them from the full list.".into())
                }
                Filter::All => ("No tools".into(), String::new()),
            }
        };
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            ui.label(egui::RichText::new(title).font(legend_font(20.0)).color(VALUE));
            ui.add_space(6.0);
            note(ui, body, LEGEND);
            ui.add_space(14.0);
            if self.filter == Filter::Installed
                && self.pending() > 0
                && self.search.trim().is_empty()
            {
                if ui.add(Key::new("install all").solid(READOUT).min_width(160.0)).clicked() {
                    acts.push(Act::InstallAll);
                }
            } else if (self.filter != Filter::All || !self.search.trim().is_empty())
                && ui.add(Key::new("show all tools").outline(VALUE).min_width(160.0)).clicked()
            {
                acts.push(Act::ShowAll);
            }
        });
    }

    fn confirm_removal(&mut self, ctx: &egui::Context, acts: &mut Vec<Act>) {
        let Some(i) = self.removing else { return };
        let tool = self.slots[i].tool;
        let modal = egui::Modal::new(Id::new("remove"))
            .backdrop_color(egui::Color32::from_black_alpha(150))
            .show(ctx, |ui| {
            ui.set_width(380.0);
            modal_title(ui, &format!("Uninstall {}?", tool.title));
            note(
                ui,
                format!(
                    "This deletes {}. Your own files are not touched, and you can install it again from here.",
                    tidy(&install::home(tool))
                ),
                LEGEND,
            );
            footer(ui, |ui| {
                if ui.add(Key::new("uninstall").solid(FAULT).min_width(120.0)).clicked() {
                    acts.push(Act::Remove(i));
                    self.removing = None;
                }
                if ui.add(Key::new("keep it").outline(VALUE).min_width(120.0)).clicked() {
                    self.removing = None;
                }
            });
        });
        if modal.should_close() {
            self.removing = None;
        }
    }

    fn drop_overlay(&self, ctx: &egui::Context) {
        let Some(hovered) = egui_bench::drop::hovering(ctx) else { return };
        let first = hovered.into_iter().next().flatten();
        let target = first.as_ref().and_then(|path| self.slot_for(path));
        let tint = if first.is_some() && target.is_none() { FAULT } else { READOUT };
        let (headline, note) = match (&first, target) {
            (Some(path), Some(i)) => {
                let slot = &self.slots[i];
                let note = if slot.exe.is_some() {
                    format!("{} will open it", slot.tool.title)
                } else {
                    format!("{} is not installed yet", slot.tool.title)
                };
                (format!("Open {}", file_label(path)), note)
            }
            (Some(path), None) => (
                format!("Nothing here opens {}", file_label(path)),
                "The tools read part files, board files, project folders, thermal images and G-code"
                    .to_string(),
            ),
            (None, _) => ("Drop to open".into(), "It goes to the tool that reads it".into()),
        };
        let glyph = target.map(|i| self.slots[i].tool).unwrap_or(&SUITE);
        egui_bench::drop::overlay(ctx, Some(icon(glyph)), &headline, &note, tint);
    }
}

fn version_line(
    slot: &Slot,
    current: Option<&str>,
    state: &State,
    release: Option<&egui_bench::update::Release>,
) -> LayoutJob {
    let installed = slot.exe.is_some();
    let line = match (installed, current) {
        (false, _) => Line::new(),
        (true, Some(v)) => Line::new().legend("installed").value(v.to_string()),
        (true, None) => Line::new().legend("installed").note("unknown version"),
    };
    let next = |line: Line| if installed { line.gap(16.0) } else { line };
    let line = match (state, release) {
        (State::Checking, _) => next(line).legend("latest").note("checking"),
        (State::Current(_), _) if current.is_some() => line.note("up to date"),
        (State::Newer(r), _) if installed && current.is_some() => {
            next(line).legend("latest").set(r.version.clone())
        }
        (_, Some(r)) if installed => next(line).legend("latest").value(r.version.clone()),
        (_, Some(r)) => line.legend("latest").value(r.version.clone()),
        _ if installed => line,
        _ => line.legend("latest").note("unknown"),
    };
    line.job()
}

fn install_all(ui: &mut Ui, key: Key, acts: &mut Vec<Act>) {
    let r = ui.add(key).on_hover_text("install every missing tool and update the ones behind");
    if r.clicked() {
        acts.push(Act::InstallAll);
    }
}

fn progress(install: &Install) -> Option<(Option<f32>, String)> {
    match install {
        Install::Fetching { got, total } if *total > 0 => Some((
            Some(*got as f32 / *total as f32),
            format!("downloading {}", fraction(*got, *total)),
        )),
        Install::Fetching { got, .. } => Some((None, format!("downloading {}", megabytes(*got)))),
        Install::Unpacking => Some((None, "installing".into())),
        _ => None,
    }
}

fn dot_ui(ui: &mut Ui, tint: Color32) {
    let (rect, _) = ui.allocate_exact_size(vec2(10.0, 16.0), Sense::hover());
    ui.painter().circle_filled(rect.center(), 3.0, tint);
}

pub(crate) fn rule(ui: &Ui, a: Pos2, b: Pos2) {
    ui.painter().line_segment([a, b], Stroke::new(1.0, ETCH));
}

pub(crate) fn face(font: FontId, color: Color32) -> TextFormat {
    TextFormat { font_id: font, color, ..Default::default() }
}

fn tracked(font: FontId, color: Color32, spacing: f32) -> TextFormat {
    TextFormat { font_id: font, color, extra_letter_spacing: spacing, ..Default::default() }
}

fn elide(ui: &Ui, text: &str, format: TextFormat, width: f32) -> std::sync::Arc<egui::Galley> {
    let mut job = LayoutJob::default();
    job.append(text, 0.0, format);
    elide_job(ui, job, width)
}

fn elide_job(ui: &Ui, mut job: LayoutJob, width: f32) -> std::sync::Arc<egui::Galley> {
    job.wrap.max_width = width.max(10.0);
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    job.wrap.overflow_character = Some('…');
    ui.fonts_mut(|f| f.layout_job(job))
}

fn search_id() -> Id {
    Id::new("search")
}

pub(crate) fn megabytes(bytes: u64) -> String {
    format!("{:.0} MB", (bytes as f64 / 1_000_000.0).max(1.0))
}

fn fraction(got: u64, total: u64) -> String {
    if total > 0 {
        format!("{:.0} of {}", got as f64 / 1_000_000.0, megabytes(total))
    } else {
        megabytes(got)
    }
}

fn ago(at: Instant) -> String {
    let s = at.elapsed().as_secs();
    match s {
        0..=59 => "just now".into(),
        60..=3599 => format!("{} min ago", s / 60),
        _ => format!("{} h ago", s / 3600),
    }
}

pub(crate) fn date(iso: &str) -> Option<String> {
    let mut parts = iso.get(..10)?.split('-');
    let (y, m, d) = (parts.next()?, parts.next()?, parts.next()?);
    let month =
        ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"]
            .get(m.parse::<usize>().ok()?.checked_sub(1)?)?;
    Some(format!("{} {month} {y}", d.parse::<u32>().ok()?))
}

pub(crate) fn skill_line(tool: &Tool, placed: &[Placed]) -> String {
    let linked: Vec<&str> =
        placed.iter().filter(|p| p.how != How::Theirs).filter_map(|p| p.agent).collect();
    match placed.iter().find(|p| p.agent.is_none()) {
        Some(p) if p.how == How::Theirs => {
            format!("your own {} skill in {} is kept", tool.name, tidy(&p.path))
        }
        Some(_) if linked.is_empty() => "agent skill installed for your agents".to_string(),
        Some(_) => format!("agent skill installed, linked for {}", linked.join(", ")),
        None => format!("agent skill linked for {}", linked.join(", ")),
    }
}

pub(crate) fn skill_hover(placed: &[Placed]) -> String {
    let mut lines: Vec<String> = placed
        .iter()
        .map(|p| {
            let how = match p.how {
                How::Copied => "installed",
                How::Linked => "linked",
                How::Theirs => "left as it was",
            };
            format!("{} {how}", tidy(&p.path))
        })
        .collect();
    lines.push(format!("read by {} and others", skills::readers(placed)));
    lines.join("\n")
}

pub(crate) fn tidy(path: &Path) -> String {
    match std::env::home_dir().and_then(|h| path.strip_prefix(h).ok().map(Path::to_path_buf)) {
        Some(rest) => format!("~/{}", rest.display()),
        None => path.display().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_release_date_reads_as_a_day() {
        assert_eq!(date("2026-10-02T09:00:00Z").as_deref(), Some("2 Oct 2026"));
        assert_eq!(date("nonsense"), None);
    }
}
