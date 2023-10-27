use eframe::{
    egui::{DragValue, Layout, Response, Sense, TextEdit, Ui, Widget},
    epaint::{Color32, Mesh, Pos2, Rect, Shape, Stroke, Vec2, Vertex, WHITE_UV},
};
use itertools::Itertools;
use std::{cmp::Ordering, collections::HashMap, iter, num::NonZeroUsize};
use thunderdome::{Arena, Index};

const BLOCK_MARGIN: f32 = 10.0;
const BLOCK_HEIGHT: f32 = 40.0;
const NOTCH_TL: Vec2 = Vec2::new(10.0, 0.0);
const NOTCH_BL: Vec2 = Vec2::new(20.0, 10.0);
const NOTCH_BR: Vec2 = Vec2::new(30.0, 10.0);
const NOTCH_TR: Vec2 = Vec2::new(40.0, 0.0);
// todo: determine from implementation?
const FILL_COLOR_LIGHT: Color32 = Color32::from_rgb(0xFF, 0xFF, 0xFF);
const FILL_COLOR_DARK: Color32 = Color32::from_rgb(0x22, 0x22, 0x22);

#[derive(Clone, Copy)]
pub enum BlockWidget {
    Label {
        text: &'static str,
    },
    TextEdit {
        key: &'static str,
        default: &'static str,
    },
    NumberEdit {
        key: &'static str,
        default: i32,
    },
}

// Terminology taken from https://en.scratch-wiki.info/wiki/Blocks#Block_Shapes
#[derive(Clone, Copy)]
pub enum BlockShape {
    /// no top notch
    Hat,
    /// top & bottom notch
    Stack,
    // TODO: Implement
    /// boolean value
    // Boolean,
    /// "normal" value
    // Reporter,
    /// top notch, `branches` branch notches, bottom notch
    C { branches: NonZeroUsize },
    /// no bottom notch
    Cap,
}

impl BlockShape {
    fn top_notch(&self) -> bool {
        !matches!(self, BlockShape::Hat)
    }

    fn bottom_notch(&self) -> bool {
        !matches!(self, BlockShape::Cap)
    }

    fn branches(&self) -> usize {
        match self {
            BlockShape::C { branches } => branches.get(),
            _ => 0,
        }
    }
}

#[derive(Clone)]
pub struct BlockDescription {
    pub shape: BlockShape,
    pub content: Vec<BlockWidget>,
}

pub trait Block {
    fn describe(&mut self) -> BlockDescription;

    fn run(&mut self);
}

struct BlockInstance {
    // TODO: why this is relative to the window?
    position: Pos2,
    last_touched_frame: u64,
    snap_target: Option<Index>,
    nexts: Vec<Option<Index>>,
    extent: Vec2,
    implementation: Box<dyn Block>,
    description: BlockDescription,
    text_data: HashMap<&'static str, String>,
    number_data: HashMap<&'static str, i32>,
}

impl BlockInstance {
    fn next(&self, index: usize) -> Option<Index> {
        self.nexts.get(index).and_then(|next| *next)
    }

    fn paint(&mut self, ui: &mut Ui, fill_color: Color32, outline_stroke: Stroke) {
        let paint_position = ui.max_rect().min;

        let main_tl = Pos2::new(paint_position.x, paint_position.y);
        let main_tr = Pos2::new(paint_position.x + self.extent.x, paint_position.y);
        let main_br = Pos2::new(
            paint_position.x + self.extent.x,
            paint_position.y + self.extent.y,
        );
        let main_bl = Pos2::new(paint_position.x, paint_position.y + self.extent.y);

        let mut top_notch_multiplier = Vec2::splat(1.0);
        let mut bottom_notch_multiplier = Vec2::splat(1.0);

        if !self.description.shape.top_notch() {
            top_notch_multiplier.y = 0.0;
        }

        if !self.description.shape.bottom_notch() {
            bottom_notch_multiplier.y = 0.0;
        }

        let mut vertices = Vec::with_capacity(12);
        let mut vertex = |pos: Pos2| {
            let index = vertices.len();
            vertices.push(Vertex {
                pos,
                uv: WHITE_UV,
                color: fill_color,
            });

            index as u32
        };

        let mut indices = Vec::with_capacity(6 * 5);
        let mut quad = |tl: u32, tr: u32, br: u32, bl: u32| {
            indices.extend_from_slice(&[tl, tr, bl, bl, br, tr]);
        };

        let mut block_part = |tl: Pos2, tr: Pos2, bl: Pos2, br: Pos2| {
            let index_tl = vertex(tl);
            vertex(tl + NOTCH_TL * top_notch_multiplier);
            vertex(tl + NOTCH_BL * top_notch_multiplier);
            vertex(tl + NOTCH_BR * top_notch_multiplier);
            vertex(tl + NOTCH_TR * top_notch_multiplier);
            vertex(tr);
            vertex(br);
            vertex(bl + NOTCH_TR * bottom_notch_multiplier);
            vertex(bl + NOTCH_BR * bottom_notch_multiplier);
            vertex(bl + NOTCH_BL * bottom_notch_multiplier);
            vertex(bl + NOTCH_TL * bottom_notch_multiplier);
            let index_bl = vertex(bl);

            for i in 0..5 {
                quad(
                    index_tl + i,
                    index_tl + 1 + i,
                    index_bl - 1 - i,
                    index_bl - i,
                );
            }
        };

        block_part(main_tl, main_tr, main_bl, main_br);
        /*
        for branch in 0..self.description.shape.branches() {
            let offset = Vec2::new(0.0, (branch + 1) as f32 * 20.0);
            block_part(
                main_tl + offset,
                main_tr + offset,
                main_bl + offset,
                main_br + offset,
            );
        }
        */

        let shape = Shape::line(
            (0..vertices.len())
                .chain(iter::once(0))
                .map(|i| vertices[i].pos)
                .collect(),
            outline_stroke,
        );

        ui.painter().add(Mesh {
            vertices,
            indices,
            ..Default::default()
        });
        ui.painter().add(shape);

        let inner = ui
            .horizontal_centered(|ui| {
                ui.add_space(BLOCK_MARGIN);

                for widget in &self.description.content {
                    let _response = match widget {
                        BlockWidget::Label { text } => ui.label(*text),
                        BlockWidget::TextEdit { key, default: _ } => ui.add(
                            TextEdit::singleline(self.text_data.get_mut(key).unwrap())
                                .desired_width(24.0)
                                .clip_text(false),
                        ),
                        BlockWidget::NumberEdit { key, default: _ } => {
                            ui.add(DragValue::new(self.number_data.get_mut(key).unwrap()))
                        }
                    };
                }

                ui.add_space(BLOCK_MARGIN);
            })
            .response;

        self.extent = inner.rect.size();
    }
}

pub struct BlockEditor {
    offset: Vec2,
    blocks: Arena<BlockInstance>,
}

impl Default for BlockEditor {
    fn default() -> BlockEditor {
        BlockEditor {
            offset: Vec2::ZERO,
            blocks: Arena::new(),
        }
    }
}

impl BlockEditor {
    pub fn add_block<B: Block + 'static>(&mut self, position: Pos2, mut block: B) {
        let description = block.describe();
        let mut text_data = HashMap::new();
        let mut number_data = HashMap::new();

        for widget in &description.content {
            match widget {
                BlockWidget::TextEdit { key, default } => {
                    text_data.insert(*key, String::from(*default));
                }
                BlockWidget::NumberEdit { key, default } => {
                    number_data.insert(*key, *default);
                }
                _ => (),
            }
        }

        self.blocks.insert(BlockInstance {
            position,
            last_touched_frame: 0,
            snap_target: None,
            nexts: match description.shape {
                BlockShape::Hat => vec![None],
                BlockShape::Stack => vec![None],
                BlockShape::C { branches } => vec![None; 1 + branches.get()],
                BlockShape::Cap => vec![],
            },
            extent: Vec2::new(0.0, BLOCK_HEIGHT),
            implementation: Box::new(block),
            description,
            text_data,
            number_data,
        });
    }
}

impl Widget for &mut BlockEditor {
    fn ui(self, ui: &mut Ui) -> Response {
        let editor_rect = ui.available_rect_before_wrap();
        let response = ui.allocate_rect(editor_rect, Sense::drag());
        if response.dragged() {
            self.offset += response.drag_delta();
        }

        ui.painter()
            .rect_filled(editor_rect, 5.0, ui.style().visuals.extreme_bg_color);

        /*
        if let Some(snap_target) = self.snap_target {
            let snap_target = &self.blocks[snap_target];

            snap_target.paint(
                ui.painter(),
                ui.max_rect().min
                    + snap_target.position.to_vec2()
                    + Vec2::new(0.0, DEPRECATED_BLOCK_SIZE.y)
                    + self.area_offset,
                &snap_target.layout(),
                ui.visuals().selection.bg_fill,
                Stroke::NONE,
            );

            self.snap_target = None;
        }
        */

        struct SetNext {
            upper_block: Index,
            upper_next_index: usize,
            next_block: Index,
        }

        let mut dragging = None;
        let mut set_next = None;
        for (index, block) in self
            .blocks
            .iter_mut()
            .sorted_unstable_by_key(|(_index, block)| block.last_touched_frame)
        {
            let child_ui_rect = Rect::from_min_size(
                ui.max_rect().min + self.offset + block.position.to_vec2(),
                block.extent,
            );

            let child_ui = ui.child_ui(child_ui_rect, Layout::default());
            {
                let mut ui = child_ui;
                ui.set_clip_rect(if block.extent.x == 0.0 {
                    Rect::NOTHING
                } else {
                    editor_rect
                });

                let response = ui.interact(child_ui_rect, ui.id().with(index), Sense::drag());

                if response.dragged() {
                    block.position += response.drag_delta();
                    // TODO: propagate this through the linked list
                    block.last_touched_frame = ui.ctx().frame_nr();
                    dragging = Some(index);
                }

                if response.drag_released() {
                    if let Some(upper_block) = block.snap_target {
                        block.snap_target = None;
                        set_next = Some(SetNext {
                            upper_block,
                            upper_next_index: 0,
                            next_block: index,
                        });
                    }
                }

                let fill_color = if ui.visuals().dark_mode {
                    FILL_COLOR_DARK
                } else {
                    FILL_COLOR_LIGHT
                };

                let outline_stroke = ui.style().interact(&response).fg_stroke;
                block.paint(&mut ui, fill_color, outline_stroke);

                // ui.label(format!("snap target: {:?}", block.snap_target.is_some()));
                // ui.label(format!("next: {:?}", block.next.is_some()));
            }
        }

        if let Some(dragging) = dragging {
            // if this block is the next of anything, clear it
            for (_index, block) in &mut self.blocks {
                for next in &mut block.nexts {
                    if *next == Some(dragging) {
                        *next = None;
                    }
                }
            }

            let notch_offset = (NOTCH_BL + NOTCH_BR) / 2.0;
            let dragging_attachment_position = self.blocks[dragging].position + notch_offset;

            // todo: support multiple nexts
            // TODO: add logic for "top" attaching
            // TODO: add logic for "between" attaching
            // TODO: add logic for "bottom of stack" attaching
            // attaches dragging block to other block
            self.blocks[dragging].snap_target = None;
            if let Some((closest, _distance)) = self
                .blocks
                .iter()
                .filter(|(index, _block)| *index != dragging)
                .filter(|(_index, block)| !block.nexts.is_empty())
                .map(|(index, other_block)| {
                    let other_attachment_position =
                        other_block.position + Vec2::new(0.0, other_block.extent.y) + notch_offset;

                    let dist = dragging_attachment_position.distance(other_attachment_position);
                    (index, dist)
                })
                .filter(|(_index, distance)| *distance < 30.0)
                .min_by(|(_index_a, a), (_index_b, b)| a.partial_cmp(b).unwrap_or(Ordering::Equal))
            {
                self.blocks[dragging].snap_target = Some(closest);
            }
        }

        if let Some(SetNext {
            upper_block,
            upper_next_index,
            next_block,
        }) = set_next
        {
            let upper_next = &mut self.blocks[upper_block].nexts[upper_next_index];
            if upper_next.is_none() {
                *upper_next = Some(next_block);
            }
        }

        // TODO: sort to avoid "tearing"
        // TODO: support multiple nexts
        let child_update: Vec<_> = self
            .blocks
            .iter()
            .filter_map(|(index, block)| block.next(0).map(|next| (index, next)))
            .collect();

        for (upper, next) in child_update {
            self.blocks[next].position =
                self.blocks[upper].position + Vec2::new(0.0, self.blocks[upper].extent.y);
        }

        response
    }
}
