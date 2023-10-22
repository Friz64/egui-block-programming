use eframe::{
    egui::{Layout, Painter, Response, Sense, Ui, Widget},
    epaint::{Color32, Mesh, Pos2, Rect, Shape, Stroke, Vec2},
};
use itertools::Itertools;
use std::{cmp::Ordering, num::NonZeroU8};
use thunderdome::{Arena, Index};

const BLOCK_SIZE: Vec2 = Vec2::new(100.0, 40.0);
const NOTCH_TL: Vec2 = Vec2::new(10.0, 0.0);
const NOTCH_BL: Vec2 = Vec2::new(20.0, 10.0);
const NOTCH_BR: Vec2 = Vec2::new(30.0, 10.0);
const NOTCH_TR: Vec2 = Vec2::new(40.0, 0.0);

// Terminology taken from https://en.scratch-wiki.info/wiki/Blocks#Block_Shapes
pub enum BlockShape {
    Hat,
    Stack,
    Boolean,
    Reporter,
    C { branches: NonZeroU8 },
    Cap,
}

pub struct BlockDescription {
    pub content: String,
    pub shape: BlockShape,
}

pub trait Block {
    // todo: should this be mutable?
    fn describe(&mut self) -> BlockDescription;
}

struct BlockInstance {
    // TODO: why this is relative to the window?
    position: Pos2,
    last_touched_frame: u64,
    snap_target: Option<Index>,
    // TODO: move into implementation? there might be multiple nexts
    next: Option<Index>,
    implementation: Box<dyn Block>,
    latest_description: BlockDescription,
}

struct BlockLayout {}

impl BlockInstance {
    fn layout(&self) -> BlockLayout {
        BlockLayout {}
    }

    fn paint(
        &self,
        painter: &Painter,
        area_offset: Vec2,
        layout: &BlockLayout,
        fill_color: Color32,
        outline_stroke: Stroke,
    ) {
        let paint_position = self.position + area_offset;
        /*
             let main_br = Pos2::new(rect.max.x, rect.max.y);
        let main_bl = Pos2::new(rect.min.x, rect.max.y);
        let main_tl = Pos2::new(rect.min.x, rect.min.y);
        let main_tr = Pos2::new(rect.max.x, rect.min.y);

        let vertices = [
            // + 0
            main_tl,
            main_tl + NOTCH_TL,
            main_tl + NOTCH_BL,
            main_tl + NOTCH_BR,
            main_tl + NOTCH_TR,
            main_tr,
            // + 6
            main_bl,
            main_bl + NOTCH_TL,
            main_bl + NOTCH_BL,
            main_bl + NOTCH_BR,
            main_bl + NOTCH_TR,
            main_br,
        ];

        let triangles = [
            // left, even
            (0, 1, 6),
            (6, 7, 1),
            // notch, going down
            (1, 2, 7),
            (7, 8, 2),
            // notch, even
            (2, 3, 8),
            (8, 9, 3),
            // notch, going up
            (3, 4, 9),
            (9, 10, 4),
            // right, even
            (4, 5, 10),
            (10, 11, 5),
        ];

        let outline_indices = [0, 1, 2, 3, 4, 5, 11, 10, 9, 8, 7, 6, 0];

        let mut block_mesh = Mesh::default();
        block_mesh.reserve_vertices(vertices.len());
        for pos in vertices {
            block_mesh.colored_vertex(pos, fill_color);
        }

        block_mesh.reserve_triangles(triangles.len());
        for (a, b, c) in triangles {
            block_mesh.add_triangle(a, b, c);
        }

        painter.add(block_mesh);
        painter.add(Shape::line(
            outline_indices.into_iter().map(|i| vertices[i]).collect(),
            outline_stroke,
        ));
            */
    }
}

pub struct BlockEditor {
    blocks: Arena<BlockInstance>,
    // TODO: try to avoid?
    snap_target: Option<Index>,
}

impl Default for BlockEditor {
    fn default() -> BlockEditor {
        BlockEditor {
            blocks: Arena::new(),
            snap_target: None,
        }
    }
}

impl BlockEditor {
    pub fn add_block<B: Block + 'static>(&mut self, position: Pos2, mut block: B) {
        let description = block.describe();
        self.blocks.insert(BlockInstance {
            position,
            last_touched_frame: 0,
            snap_target: None,
            next: None,
            implementation: Box::new(block),
            latest_description: description,
        });
    }
}

// TODO: clean up
// TODO: add logic for "top" attaching
// TODO: add logic for "between" attaching
// TODO: add logic for "bottom of stack" attaching
impl Widget for &mut BlockEditor {
    fn ui(self, ui: &mut Ui) -> Response {
        let editor_rect = ui.available_rect_before_wrap();
        let response = ui.allocate_rect(editor_rect, Sense::drag());
        if response.dragged() {
            for (_index, block) in &mut self.blocks {
                // TOOD: this is not how this works
                block.position += response.drag_delta();
            }
        }

        ui.painter()
            .rect_filled(editor_rect, 5.0, ui.style().visuals.extreme_bg_color);

        if let Some(snap_target) = self.snap_target {
            let snap_target = &self.blocks[snap_target];

            paint_block_mesh(
                ui.painter(),
                Rect::from_min_size(
                    ui.max_rect().min
                        + snap_target.position.to_vec2()
                        + Vec2::new(0.0, BLOCK_SIZE.y),
                    BLOCK_SIZE,
                ),
                ui.visuals().selection.bg_fill,
                Stroke::NONE,
            );

            self.snap_target = None;
        }

        let mut dragging = None;
        let mut set_next = None; // todo: cleanup?
        for (index, block) in self
            .blocks
            .iter_mut()
            .sorted_unstable_by_key(|(_index, block)| block.last_touched_frame)
        {
            let child_ui_rect =
                Rect::from_min_size(ui.max_rect().min + block.position.to_vec2(), BLOCK_SIZE);

            let child_ui = ui.child_ui(child_ui_rect, Layout::default());
            {
                let mut ui = child_ui;
                ui.set_clip_rect(editor_rect);

                let response = ui.interact(child_ui_rect, ui.id().with(index), Sense::drag());

                if response.dragged() {
                    block.position += response.drag_delta();
                    // TODO: propagate this through the linked list
                    block.last_touched_frame = ui.ctx().frame_nr();
                    dragging = Some(index);
                    self.snap_target = block.snap_target;
                }

                if response.drag_released() {
                    if let Some(snap_target) = block.snap_target {
                        set_next = Some((snap_target, index));
                        block.snap_target = None;
                    }
                }

                paint_block_mesh(
                    ui.painter(),
                    ui.max_rect(),
                    Color32::from_rgb(0x22, 0x22, 0x22),
                    ui.style().interact(&response).fg_stroke,
                );

                ui.label(block.implementation.describe().content);
                ui.label(format!("snap target: {:?}", block.snap_target.is_some()));
                ui.label(format!("next: {:?}", block.next.is_some()));
            }
        }

        if let Some(dragging) = dragging {
            for (_index, block) in &mut self.blocks {
                if block.next == Some(dragging) {
                    block.next = None;
                }
            }

            let position = {
                let block = &mut self.blocks[dragging];
                block.snap_target = None;
                let top_attachment_offset = Vec2::new(
                    (NOTCH_BL.x + NOTCH_BR.x) / 2.0,
                    (NOTCH_BL.y + NOTCH_BR.y) / 2.0,
                );
                block.position + top_attachment_offset
            };

            if let Some((closest, _distance)) = self
                .blocks
                .iter()
                .filter(|(index, _block)| *index != dragging)
                .map(|(index, other_block)| {
                    let bottom_attachment_offset = Vec2::new(
                        (NOTCH_BL.x + NOTCH_BR.x) / 2.0,
                        BLOCK_SIZE.y + (NOTCH_BL.y + NOTCH_BR.y) / 2.0,
                    );

                    let dist = (position).distance(other_block.position + bottom_attachment_offset);
                    (index, dist)
                })
                .filter(|(_index, distance)| *distance < 30.0)
                .min_by(|(_index_a, a), (_index_b, b)| a.partial_cmp(b).unwrap_or(Ordering::Equal))
            {
                self.blocks[dragging].snap_target = Some(closest);
            }
        }

        if let Some((upper, next)) = set_next {
            let upper_next = &mut self.blocks[upper].next;
            if upper_next.is_none() {
                *upper_next = Some(next);
            }
        }

        // TODO: sort to avoid "tearing"
        let child_update: Vec<_> = self
            .blocks
            .iter()
            .filter_map(|(index, block)| block.next.map(|next| (index, next)))
            .collect();

        for (upper, next) in child_update {
            self.blocks[next].position = self.blocks[upper].position + Vec2::new(0.0, BLOCK_SIZE.y);
        }

        response
    }
}

fn paint_block_mesh(painter: &Painter, rect: Rect, fill_color: Color32, outline_stroke: Stroke) {
    let main_br = Pos2::new(rect.max.x, rect.max.y);
    let main_bl = Pos2::new(rect.min.x, rect.max.y);
    let main_tl = Pos2::new(rect.min.x, rect.min.y);
    let main_tr = Pos2::new(rect.max.x, rect.min.y);

    let vertices = [
        // + 0
        main_tl,
        main_tl + NOTCH_TL,
        main_tl + NOTCH_BL,
        main_tl + NOTCH_BR,
        main_tl + NOTCH_TR,
        main_tr,
        // + 6
        main_bl,
        main_bl + NOTCH_TL,
        main_bl + NOTCH_BL,
        main_bl + NOTCH_BR,
        main_bl + NOTCH_TR,
        main_br,
    ];

    let triangles = [
        // left, even
        (0, 1, 6),
        (6, 7, 1),
        // notch, going down
        (1, 2, 7),
        (7, 8, 2),
        // notch, even
        (2, 3, 8),
        (8, 9, 3),
        // notch, going up
        (3, 4, 9),
        (9, 10, 4),
        // right, even
        (4, 5, 10),
        (10, 11, 5),
    ];

    let outline_indices = [0, 1, 2, 3, 4, 5, 11, 10, 9, 8, 7, 6, 0];

    let mut block_mesh = Mesh::default();
    block_mesh.reserve_vertices(vertices.len());
    for pos in vertices {
        block_mesh.colored_vertex(pos, fill_color);
    }

    block_mesh.reserve_triangles(triangles.len());
    for (a, b, c) in triangles {
        block_mesh.add_triangle(a, b, c);
    }

    painter.add(block_mesh);
    painter.add(Shape::line(
        outline_indices.into_iter().map(|i| vertices[i]).collect(),
        outline_stroke,
    ));
}
