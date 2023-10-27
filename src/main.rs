mod block;

use block::{Block, BlockDescription, BlockEditor, BlockShape, BlockWidget};
use eframe::{egui, epaint::Pos2};
use std::num::NonZeroUsize;

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
        block_editor.add_block(Pos2::new(50.0, 50.0), TestingBlock {});
        block_editor.add_block(Pos2::new(100.0, 100.0), IfBlock {});

        Self { block_editor }
    }
}

impl eframe::App for Main {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::Window::new("My Window").show(ctx, |ui| {
            ui.label("Hello World!");
            egui::widgets::global_dark_light_mode_buttons(ui);

            if ui.button("stack").clicked() {
                self.block_editor
                    .add_block(Pos2::new(150.0, 150.0), TestingBlock {});
            }

            ui.add(&mut self.block_editor);
        });
    }
}

struct TestingBlock {}

impl TestingBlock {
    const STEPS: &'static str = "steps";
    const TESTING: &'static str = "testing";
}

impl Block for TestingBlock {
    fn describe(&mut self) -> BlockDescription {
        BlockDescription {
            shape: BlockShape::Stack,
            content: vec![vec![
                BlockWidget::Label { text: "move" },
                BlockWidget::NumberEdit {
                    key: Self::STEPS,
                    default: 0,
                },
                BlockWidget::Label { text: "steps" },
                BlockWidget::TextEdit {
                    key: Self::TESTING,
                    default: ":3",
                },
                BlockWidget::Label { text: "abc" },
            ]],
        }
    }

    fn run(&mut self) {}
}

struct IfBlock {}

impl Block for IfBlock {
    fn describe(&mut self) -> BlockDescription {
        BlockDescription {
            shape: BlockShape::C {
                branches: NonZeroUsize::new(3).unwrap(),
            },
            content: vec![
                vec![
                    BlockWidget::Label { text: "move" },
                    BlockWidget::NumberEdit {
                        key: "steps0",
                        default: 0,
                    },
                    BlockWidget::Label { text: "steps" },
                    BlockWidget::TextEdit {
                        key: "testing0",
                        default: ":3",
                    },
                    BlockWidget::Label { text: "abc" },
                ],
                vec![
                    BlockWidget::Label { text: "move" },
                    BlockWidget::NumberEdit {
                        key: "steps1",
                        default: 0,
                    },
                    BlockWidget::Label { text: "steps" },
                    BlockWidget::TextEdit {
                        key: "testing1",
                        default: ":3",
                    },
                    BlockWidget::Label { text: "abc" },
                ],
                vec![
                    BlockWidget::Label { text: "move" },
                    BlockWidget::NumberEdit {
                        key: "steps2",
                        default: 0,
                    },
                    BlockWidget::Label { text: "steps" },
                    BlockWidget::TextEdit {
                        key: "testing2",
                        default: ":3",
                    },
                    BlockWidget::Label { text: "abc" },
                ],
                vec![
                    BlockWidget::Label { text: "move" },
                    BlockWidget::NumberEdit {
                        key: "steps3",
                        default: 0,
                    },
                    BlockWidget::Label { text: "steps" },
                    BlockWidget::TextEdit {
                        key: "testing3",
                        default: ":3",
                    },
                    BlockWidget::Label { text: "abc" },
                ],
            ],
        }
    }

    fn run(&mut self) {}
}
