use eframe::{
    egui::{DragValue, Layout, Response, Sense, TextEdit, Ui, Widget},
    epaint::{Color32, Mesh, Pos2, Rect, Shape, Vec2, Vertex, WHITE_UV},
};
use itertools::Itertools;
use std::{cmp::Ordering, collections::HashMap, iter, num::NonZeroUsize};
use thunderdome::{Arena, Index};

const PART_PADDING: f32 = 10.0;
const PART_HEIGHT_MIN: f32 = 40.0;
const MULTIPART_INDENT: f32 = 15.0;
const EMPTY_BRANCH_HEIGHT: f32 = 20.0;
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
    /// parts -> widgets
    pub content: Vec<Vec<BlockWidget>>,
}

pub trait Block {
    fn describe(&mut self) -> BlockDescription;

    fn run(&mut self);
}

enum Next {
    NotApplicable,
    None,
    Some { index: Index, height: f32 },
}

struct BlockPart {
    top_offset: Vec2,
    bottom_offset: Vec2,
    width: f32,
    next: Next,
}

impl BlockPart {
    fn height(&self) -> f32 {
        self.bottom_offset.y - self.top_offset.y
    }

    fn extent(&self) -> Vec2 {
        Vec2 {
            x: self.width,
            y: self.height(),
        }
    }
}

struct BlockInstance {
    position: Pos2,
    last_touched_frame: u64,
    snap_target: Option<Index>,
    parts: Vec<BlockPart>,
    _implementation: Box<dyn Block>,
    description: BlockDescription,
    text_data: HashMap<&'static str, String>,
    number_data: HashMap<&'static str, i32>,
}

impl BlockInstance {
    fn total_height(&self) -> f32 {
        self.parts.last().unwrap().bottom_offset.y
    }

    fn paint(&mut self, mut uis: Vec<Ui>, response: &Response) {
        let widget_visuals = uis[0].style().interact(response);
        let fill_color = if uis[0].visuals().dark_mode {
            FILL_COLOR_DARK
        } else {
            FILL_COLOR_LIGHT
        };

        let mut vertices = Vec::with_capacity(12 * self.parts.len());
        let mut vertex = |pos: Pos2| {
            let index = vertices.len();
            vertices.push(Vertex {
                pos,
                uv: WHITE_UV,
                color: fill_color,
            });

            index as u32
        };

        let mut indices = Vec::with_capacity(6 * 5 * self.parts.len());
        let mut quad = |tl: u32, tr: u32, br: u32, bl: u32| {
            indices.extend_from_slice(&[tl, tr, bl, bl, br, tr]);
        };

        let mut top_notch_multiplier = Vec2::splat(1.0);
        let mut bottom_notch_multiplier = Vec2::splat(1.0);

        if !self.description.shape.top_notch() {
            top_notch_multiplier.y = 0.0;
        }

        if !self.description.shape.bottom_notch() {
            bottom_notch_multiplier.y = 0.0;
        }

        let mut side_top = None;
        let mut side_bottom = None;
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

            side_bottom = Some((index_bl, index_tl));
            if side_top.is_none() {
                side_top = Some((index_tl, index_bl));
            }

            for i in 0..5 {
                quad(
                    index_tl + i,
                    index_tl + 1 + i,
                    index_bl - 1 - i,
                    index_bl - i,
                );
            }
        };

        let paint_position = uis[0].max_rect().min;
        for part in &self.parts {
            block_part(
                paint_position + part.top_offset,
                paint_position + part.top_offset + Vec2::new(part.width, 0.0),
                paint_position + part.bottom_offset,
                paint_position + part.top_offset + Vec2::new(part.width, part.height()),
            );
        }

        if self.parts.len() > 1 {
            let (tl, tr) = side_top.unwrap();
            let (bl, br) = side_bottom.unwrap();
            quad(tl, tr, br, bl);
        }

        let shape = Shape::line(
            (0..vertices.len())
                .chain(iter::once(0))
                .map(|i| vertices[i].pos)
                .collect(),
            widget_visuals.fg_stroke,
        );

        uis[0].painter().add(Mesh {
            vertices,
            indices,
            ..Default::default()
        });
        uis[0].painter().add(shape);

        for (i, (part, ui)) in self.parts.iter_mut().zip(uis.iter_mut()).enumerate() {
            let content = ui
                .horizontal_centered(|ui| {
                    ui.add_space(PART_PADDING);

                    for widget in &self.description.content[i] {
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

                    ui.add_space(PART_PADDING);
                })
                .response;

            let size = content.rect.size();
            part.width = size.x;
            part.bottom_offset.y = part.top_offset.y + size.y;
        }
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
        assert_eq!(
            description.content.len(),
            description.shape.branches() + 1,
            "number of parts does not match number of branches"
        );

        let mut text_data = HashMap::new();
        let mut number_data = HashMap::new();
        for part in &description.content {
            for widget in part {
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
        }

        let parts = description
            .content
            .iter()
            .enumerate()
            .map(|(i, _part)| BlockPart {
                top_offset: Vec2::ZERO,
                bottom_offset: Vec2::new(0.0, PART_HEIGHT_MIN),
                width: 0.0,
                next: if description.shape.bottom_notch() || i < description.content.len() - 1 {
                    Next::None
                } else {
                    Next::NotApplicable
                },
            })
            .collect();

        self.blocks.insert(BlockInstance {
            position,
            last_touched_frame: 0,
            snap_target: None,
            parts,
            _implementation: Box::new(block),
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
            upper_part: usize,
            next_block: Index,
        }

        let mut dragging = None;
        let mut set_next = None;
        for (index, block) in self
            .blocks
            .iter_mut()
            .sorted_unstable_by_key(|(_index, block)| block.last_touched_frame)
        {
            let sense = Sense::drag();
            let part_count = block.parts.len();
            let mut uis = Vec::with_capacity(block.parts.len());
            let mut y_offset = 0.0;
            let mut responses_union: Option<Response> = None;
            for (i, part) in block.parts.iter_mut().enumerate() {
                let part_height = part.height();

                let last = part_count - 1;
                part.top_offset = Vec2::new(if i != 0 { MULTIPART_INDENT } else { 0.0 }, y_offset);
                part.bottom_offset = Vec2::new(
                    if i != last { MULTIPART_INDENT } else { 0.0 },
                    y_offset + part_height,
                );

                let child_ui_rect = Rect::from_min_size(
                    ui.max_rect().min + self.offset + block.position.to_vec2() + part.top_offset,
                    part.extent(),
                );

                let mut ui = ui.child_ui(child_ui_rect, Layout::default());
                ui.set_clip_rect(if part.width == 0.0 {
                    Rect::NOTHING
                } else {
                    editor_rect
                });

                let response = ui.interact(child_ui_rect, ui.id().with(index).with(i), sense);
                responses_union = Some(if let Some(other_responses) = responses_union {
                    other_responses.union(response)
                } else {
                    response
                });

                y_offset += part.height();
                if let Next::Some { height, .. } = part.next {
                    y_offset += height;
                } else {
                    y_offset += EMPTY_BRANCH_HEIGHT;
                }

                uis.push(ui);
            }

            let mut response = responses_union.unwrap();
            if part_count > 1 {
                response = response.union(ui.interact(
                    Rect {
                        min: uis.first().unwrap().max_rect().left_bottom(),
                        max: uis.last().unwrap().max_rect().left_bottom(),
                    },
                    ui.id().with(index).with("side"),
                    sense,
                ));
            }

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
                        upper_part: 0,
                        next_block: index,
                    });
                }
            }

            block.paint(uis, &response);
        }

        if let Some(dragging) = dragging {
            // if this block is the next of anything, clear it
            for (_index, block) in &mut self.blocks {
                for part in &mut block.parts {
                    if matches!(part.next, Next::Some { index, .. } if index == dragging) {
                        part.next = Next::None;
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
                .map(|(index, other_block)| {
                    let other_attachment_position = other_block.position
                        + other_block.parts.last().unwrap().bottom_offset
                        + notch_offset;

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
            upper_part,
            next_block,
        }) = set_next
        {
            let upper_next = &mut self.blocks[upper_block].parts[upper_part].next;
            if matches!(upper_next, Next::None) {
                *upper_next = Next::Some {
                    index: next_block,
                    height: 0.0,
                };
            }
        }

        // TODO: sort to avoid "tearing"
        // TODO: support multiple nexts
        let child_update: Vec<_> = self
            .blocks
            .iter()
            .filter_map(|(index, block)| match block.parts[0].next {
                Next::Some { index: next, .. } => Some((index, next)),
                _ => None,
            })
            .collect();

        for (upper, next) in child_update {
            self.blocks[next].position = self.blocks[upper].position
                + self.blocks[upper].parts.last().unwrap().bottom_offset;
        }

        response
    }
}
