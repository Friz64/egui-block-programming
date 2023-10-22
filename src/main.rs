mod block;

use block::{Block, BlockDescription, BlockEditor, BlockShape};
use eframe::{egui, epaint::Pos2};

fn main() -> Result<(), eframe::Error> {
    std::env::set_var("WINIT_UNIX_BACKEND", "x11");

    eframe::run_native(
        "My egui App",
        eframe::NativeOptions {
            ..Default::default()
        },
        Box::new(|_cc| Box::<Main>::default()),
    )
}

struct Main {
    block_editor: BlockEditor,
}

impl Default for Main {
    fn default() -> Self {
        let mut block_editor = BlockEditor::default();
        block_editor.add_block(Pos2::new(50.0, 50.0), TestingBlock { counter: 0 });
        block_editor.add_block(Pos2::new(150.0, 150.0), TestingBlock { counter: 0 });

        Self { block_editor }
    }
}

impl eframe::App for Main {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::Window::new("My Window").show(ctx, |ui| {
            ui.label("Hello World!");
            egui::widgets::global_dark_light_mode_buttons(ui);

            if ui.button("add").clicked() {
                self.block_editor
                    .add_block(Pos2::new(150.0, 150.0), TestingBlock { counter: 0 });
            }

            ui.add(&mut self.block_editor);
        });
    }
}

struct TestingBlock {
    counter: usize,
}

impl Block for TestingBlock {
    fn describe(&mut self) -> BlockDescription {
        self.counter += 1;
        BlockDescription {
            content: format!("count {} :3", self.counter),
            shape: BlockShape::Stack,
        }
    }
}
