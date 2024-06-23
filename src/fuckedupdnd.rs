use eframe::{
    egui::{self, CursorIcon, Id, InnerResponse, Label, LayerId, Order, Sense, Ui},
    epaint::{self, vec2, Rect, Shape, Vec2},
};

fn main() -> Result<(), eframe::Error> {
    eframe::run_native(
        "My egui App",
        eframe::NativeOptions {
            ..Default::default()
        },
        Box::new(|_cc| Box::<Main>::default()),
    )
}

struct Main {
    columns: Vec<Vec<String>>,
}

impl Default for Main {
    fn default() -> Self {
        Self {
            columns: vec![
                vec!["Item A", "Item B", "Item C"],
                vec!["Item D", "Item E"],
                vec!["Item F", "Item G", "Item H"],
            ]
            .into_iter()
            .map(|v| v.into_iter().map(ToString::to_string).collect())
            .collect(),
        }
    }
}

pub fn drag_source(ui: &mut Ui, id: Id, body: impl FnOnce(&mut Ui)) {
    if ui.memory(|mem| mem.is_being_dragged(id)) {
        ui.ctx().set_cursor_icon(CursorIcon::Grabbing);

        // Paint the body to a new layer:
        let layer_id = LayerId::new(Order::Tooltip, id);
        let response = ui.with_layer_id(layer_id, body).response;

        // Now we move the visuals of the body to where the mouse is.
        // Normally you need to decide a location for a widget first,
        // because otherwise that widget cannot interact with the mouse.
        // However, a dragged component cannot be interacted with anyway
        // (anything with `Order::Tooltip` always gets an empty [`Response`])
        // So this is fine!

        if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
            let delta = pointer_pos - response.rect.center();
            ui.ctx().translate_layer(layer_id, delta);
        }
    } else {
        ui.horizontal(|ui| {
            let drag = ui
                .scope(|ui| {
                    ui.label("=");
                })
                .response;

            body(ui);

            // Check for drags:
            let response = ui.interact(drag.rect, id, Sense::drag());
            if response.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::Grab);
            }
        });
    }
}

pub fn drop_target<R>(
    ui: &mut Ui,
    can_accept_what_is_being_dragged: bool,
    body: impl FnOnce(&mut Ui) -> R,
) -> InnerResponse<R> {
    let is_being_dragged = ui.memory(|mem| mem.is_anything_being_dragged());

    let margin = Vec2::splat(4.0);

    let outer_rect_bounds = ui.available_rect_before_wrap();
    let inner_rect = outer_rect_bounds.shrink2(margin);
    let where_to_put_background = ui.painter().add(Shape::Noop);
    let mut content_ui = ui.child_ui(inner_rect, *ui.layout());
    let ret = body(&mut content_ui);
    let outer_rect = Rect::from_min_max(outer_rect_bounds.min, content_ui.min_rect().max + margin);
    let (rect, response) = ui.allocate_at_least(outer_rect.size(), Sense::hover());

    let style = if is_being_dragged && can_accept_what_is_being_dragged && response.hovered() {
        ui.visuals().widgets.active
    } else {
        ui.visuals().widgets.inactive
    };

    let mut fill = style.bg_fill;
    let mut stroke = style.bg_stroke;
    if is_being_dragged && !can_accept_what_is_being_dragged {
        fill = ui.visuals().gray_out(fill);
        stroke.color = ui.visuals().gray_out(stroke.color);
    }

    ui.painter().set(
        where_to_put_background,
        epaint::RectShape::new(rect, style.rounding, fill, stroke),
    );

    InnerResponse::new(ret, response)
}

impl eframe::App for Main {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // egui::SidePanel::left("my_left_panel").show(ctx, |ui| {
        // ui.label("Hello World!");
        // });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("This is a proof-of-concept of drag-and-drop in egui.");
            ui.label("Drag items between columns.");

            let id_source = "my_drag_and_drop_demo";
            let mut source_col_row = None;
            let mut drop_col = None;
            ui.columns(self.columns.len(), |uis| {
                for (col_idx, column) in self.columns.clone().into_iter().enumerate() {
                    let ui = &mut uis[col_idx];
                    let can_accept_what_is_being_dragged = true; // We accept anything being dragged (for now) ¯\_(ツ)_/¯
                    let inner_response = drop_target(ui, can_accept_what_is_being_dragged, |ui| {
                        let content = ui.scope(|ui| {
                            ui.set_min_size(vec2(64.0, 100.0));
                            for (row_idx, item) in column.iter().enumerate() {
                                let item_id = Id::new(id_source).with(col_idx).with(row_idx);
                                drag_source(ui, item_id, |ui| {
                                    let response = ui.add(Label::new(item).sense(Sense::click()));
                                    response.context_menu(|ui| {
                                        if ui.button("Remove").clicked() {
                                            self.columns[col_idx].remove(row_idx);
                                            ui.close_menu();
                                        }
                                    });
                                });

                                if ui.memory(|mem| mem.is_being_dragged(item_id)) {
                                    source_col_row = Some((col_idx, row_idx));
                                }
                            }
                        });

                        content.response.clicked_elsewhere()
                    });

                    let clicked_elsewhere = inner_response.inner;
                    let response = inner_response.response.context_menu(|ui| {
                        if !clicked_elsewhere {
                            ui.close_menu();
                        }

                        if ui.button("New Item").clicked() {
                            self.columns[col_idx].push("New Item".to_owned());
                            ui.close_menu();
                        }
                    });

                    let is_being_dragged = ui.memory(|mem| mem.is_anything_being_dragged());
                    if is_being_dragged && can_accept_what_is_being_dragged && response.hovered() {
                        drop_col = Some(col_idx);
                    }
                }
            });

            if let Some((source_col, source_row)) = source_col_row {
                if let Some(drop_col) = drop_col {
                    if ui.input(|i| i.pointer.any_released()) {
                        // do the drop:
                        let item = self.columns[source_col].remove(source_row);
                        self.columns[drop_col].push(item);
                    }
                }
            }
        });
    }
}
