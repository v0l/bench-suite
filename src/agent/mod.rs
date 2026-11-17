mod backend;
mod local;
mod serve;
mod tools;

use crate::app::Hub;
use agent_ui::Agent;
pub use backend::Suite;
use std::sync::Arc;

pub type Page = Agent<Hub>;

pub fn new(ctx: &egui::Context, open: bool) -> Page {
    Agent::new(ctx, Arc::new(Suite::default()), open)
}

pub fn run_helper() -> Option<i32> {
    agent_core::sandbox::run_helper()
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_ui::chat::{Attached, Turn};
    use egui_kittest::Harness;

    fn page() -> Harness<'static, Page> {
        page_at([900.0, 700.0])
    }

    fn page_at(size: [f32; 2]) -> Harness<'static, Page> {
        let ctx = egui::Context::default();
        let mut agent = Agent::new(&ctx, Arc::new(Suite::default()), true);
        agent.chat.config.model = "qwen3".into();
        agent.chat.turns.push(Turn::You {
            text: "Which tools are installed?".into(),
            files: Vec::<Attached>::new(),
        });
        agent.chat.turns.push(Turn::Thinking("I should list the tools first.".into()));
        agent.chat.turns.push(Turn::Did {
            name: "list_tools".into(),
            args: "{}".into(),
            answer: Some(Ok("gcad, agentee".into())),
        });
        agent.chat.turns.push(Turn::Said("gcad and agentee are installed.".into()));
        let mut setup = egui_kittest::wgpu::default_wgpu_setup();
        if let egui_wgpu::WgpuSetup::CreateNew(new) = &mut setup {
            new.instance_descriptor.flags = egui_wgpu::wgpu::InstanceFlags::empty();
        }
        let mut h = Harness::builder().with_size(size).wgpu_setup(setup).build_ui_state(
            |ui, agent: &mut Page| {
                egui_bench::theme::install(ui.ctx());
                agent.show(ui);
            },
            agent,
        );
        h.run_steps(4);
        h
    }

    fn painted(h: &Harness<'_, Page>) -> String {
        fn walk(s: &egui::Shape, out: &mut String) {
            match s {
                egui::Shape::Text(t) => {
                    out.push_str(t.galley.text());
                    out.push('\n');
                }
                egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
                _ => {}
            }
        }
        let mut out = String::new();
        for c in &h.output().shapes {
            walk(&c.shape, &mut out);
        }
        out.to_lowercase()
    }

    #[test]
    fn the_chat_shows_a_tool_row_and_reasoning_through_the_shared_widgets() {
        let h = page();
        let text = painted(&h);
        for words in ["which tools are installed", "list_tools", "gcad and agentee are installed"] {
            assert!(text.contains(words), "missing {words:?} in:\n{text}");
        }
        assert!(text.contains("thinking") || text.contains("list the tools"), "no reasoning block:\n{text}");
    }

    #[test]
    fn the_chat_renders_an_image() {
        for width in [1280.0, 900.0, 420.0] {
            let mut h = page_at([width, 700.0]);
            let image = h.render().expect("render");
            image.save(std::env::temp_dir().join(format!("chat-after-{width}.png"))).expect("save");
        }
    }
}
