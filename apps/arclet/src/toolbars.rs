//! Arclet's standard toolbar row and left main toolbar. Each button starts one engine command
//! id, or runs the same file and edit action the menus use, so this module holds no CAD logic.
//! Every icon is drawn here with painter shapes; none is taken from another product.

use std::f32::consts::{PI, TAU};
use std::time::Duration;

use egui::{Color32, Id, Painter, Rect, Shape, Stroke, Ui, pos2, vec2};
use solidcraft_ui_egui::SolveApp;

use crate::classic_look::chrome;
use solidcraft_ui_egui::i18n::tr;
use solidcraft_ui_egui::theme::Tokens;
use solidcraft_ui_egui::toolbar::FileAction;
use solidcraft_ui_egui::ui_kit::{self, Container, Edge, Surface};

const STANDARD_BUTTON: f32 = chrome::STANDARD_BUTTON;
const MAIN_WIDTH: f32 = chrome::MAIN_WIDTH;
const MAIN_BUTTON: f32 = chrome::MAIN_BUTTON;
const ICON: f32 = chrome::ICON_SIZE;
/// How long a press must last before a button with a flyout opens it.
const HOLD_SECONDS: f64 = 0.45;

/// What a toolbar button asks the app to do. The app runs it after the frame is drawn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pick {
    /// An engine command id, started the same way the menus start it.
    Start(&'static str),
    /// A shared file or edit action, run the same way the File and Edit menus run it.
    File(FileAction),
}

/// Original icon shapes, drawn with painter lines and circles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Glyph {
    NewPage,
    Folder,
    Disk,
    Undo,
    Redo,
    Repeat,
    Pointer,
    Trash,
    Move,
    Scale,
    ZoomExtents,
    Iso,
    Layout,
    Line,
    Circle,
    Arc,
    Rectangle,
    Spline,
    Extrude,
    Loft,
    Sweep,
    Revolve,
    Boolean,
    Fillet,
    Trim,
    Offset,
    Mirror,
    Measure,
    Zebra,
    Curvature,
}

/// A button and the related commands in its flyout. A flyout needs three commands in all,
/// so a button with fewer than two related commands has no corner triangle.
struct Group {
    label: &'static str,
    id: &'static str,
    glyph: Glyph,
    more: &'static [(&'static str, &'static str)],
}

const STANDARD: &[&[Group]] = &[
    &[
        Group { label: "arclet.new", id: "file.new", glyph: Glyph::NewPage, more: &[] },
        Group { label: "arclet.open", id: "doc.open", glyph: Glyph::Folder, more: &[] },
        Group { label: "arclet.save", id: "file.save", glyph: Glyph::Disk, more: &[] },
    ],
    &[
        Group { label: "arclet.undo", id: "edit.undo", glyph: Glyph::Undo, more: &[] },
        Group { label: "arclet.redo", id: "edit.redo", glyph: Glyph::Redo, more: &[] },
        Group { label: "arclet.repeat", id: "edit.repeat", glyph: Glyph::Repeat, more: &[] },
    ],
    &[
        Group { label: "arclet.select_objects", id: "select.set", glyph: Glyph::Pointer, more: &[] },
        Group { label: "arclet.delete", id: "selection.delete", glyph: Glyph::Trash, more: &[] },
        Group { label: "arclet.move_rotate", id: "solid.move", glyph: Glyph::Move, more: &[] },
        Group { label: "arclet.scale", id: "solid.scale", glyph: Glyph::Scale, more: &[] },
    ],
    &[
        Group { label: "arclet.fit", id: "view.fit", glyph: Glyph::ZoomExtents, more: &[] },
        Group {
            label: "arclet.iso",
            id: "view.isometric",
            glyph: Glyph::Iso,
            more: &[("arclet.perspective", "view.perspective"), ("arclet.top", "view.top"), ("arclet.front", "view.front")],
        },
        Group { label: "arclet.layout", id: "view.layout", glyph: Glyph::Layout, more: &[] },
    ],
];

const MAIN: &[&[Group]] = &[
    &[
        Group {
            label: "arclet.line",
            id: "sketch.line",
            glyph: Glyph::Line,
            more: &[("arclet.toolbar.midline", "sketch.line.midpoint"), ("arclet.toolbar.line3d", "sketch.line3d")],
        },
        Group {
            label: "arclet.circle",
            id: "sketch.circle.center",
            glyph: Glyph::Circle,
            more: &[
                ("arclet.toolbar.circle_two_point", "sketch.circle.two_point"),
                ("arclet.toolbar.circle_three_point", "sketch.circle.three_point"),
                ("arclet.toolbar.circle_two_tangent", "sketch.circle.two_tangent"),
            ],
        },
        Group {
            label: "arclet.arc_three_point",
            id: "sketch.arc.three_point",
            glyph: Glyph::Arc,
            more: &[("arclet.arc_center", "sketch.arc.center_point"), ("arclet.arc_tangent", "sketch.arc.tangent")],
        },
        Group {
            label: "arclet.rectangle",
            id: "sketch.rectangle.two_point",
            glyph: Glyph::Rectangle,
            more: &[("arclet.toolbar.rect_center", "sketch.rectangle.center"), ("arclet.toolbar.rect_rounded", "sketch.rectangle.rounded")],
        },
        Group {
            label: "arclet.spline",
            id: "sketch.spline.control_point",
            glyph: Glyph::Spline,
            more: &[("arclet.spline_fit", "sketch.spline.fit_point"), ("arclet.spline_five", "sketch.spline.control_point_5")],
        },
    ],
    &[
        Group { label: "arclet.extrude", id: "solid.extrude", glyph: Glyph::Extrude, more: &[] },
        Group {
            label: "arclet.loft",
            id: "solid.loft",
            glyph: Glyph::Loft,
            more: &[("arclet.toolbar.surface_loft", "surface.loft"), ("arclet.ruled", "surface.ruled")],
        },
        Group {
            label: "arclet.sweep",
            id: "solid.sweep",
            glyph: Glyph::Sweep,
            more: &[("arclet.toolbar.surface_sweep", "surface.sweep"), ("arclet.pipe", "solid.pipe")],
        },
        Group { label: "arclet.revolve", id: "solid.revolve", glyph: Glyph::Revolve, more: &[] },
    ],
    &[
        Group {
            label: "arclet.combine",
            id: "solid.combine",
            glyph: Glyph::Boolean,
            more: &[("arclet.toolbar.split_body", "solid.split_body"), ("arclet.toolbar.split_face", "solid.split_face")],
        },
        Group {
            label: "arclet.fillet",
            id: "solid.fillet",
            glyph: Glyph::Fillet,
            more: &[("arclet.chamfer", "solid.chamfer"), ("arclet.toolbar.sketch_fillet", "sketch.fillet")],
        },
        Group {
            label: "arclet.trim",
            id: "sketch.trim",
            glyph: Glyph::Trim,
            more: &[("arclet.toolbar.trim_path", "sketch.trim.path"), ("arclet.toolbar.surface_trim", "surface.trim")],
        },
        Group {
            label: "arclet.offset",
            id: "sketch.offset",
            glyph: Glyph::Offset,
            more: &[("arclet.toolbar.surface_offset", "surface.offset"), ("arclet.toolbar.surface_thicken", "surface.thicken")],
        },
        Group { label: "arclet.mirror", id: "solid.mirror", glyph: Glyph::Mirror, more: &[] },
    ],
    &[
        Group {
            label: "arclet.measure",
            id: "inspect.measure",
            glyph: Glyph::Measure,
            more: &[("arclet.toolbar.clearance", "inspect.clearance"), ("arclet.toolbar.center_of_mass", "inspect.center_of_mass")],
        },
        Group {
            label: "arclet.zebra",
            id: "inspect.zebra",
            glyph: Glyph::Zebra,
            more: &[("arclet.environment", "inspect.environment_map"), ("arclet.continuity", "inspect.continuity")],
        },
        Group {
            label: "arclet.curvature",
            id: "inspect.curvature_map",
            glyph: Glyph::Curvature,
            more: &[("arclet.comb", "inspect.curvature_comb"), ("arclet.toolbar.minimum_radius", "inspect.minimum_radius")],
        },
    ],
];

/// Flyout and press state for the two toolbars. The app keeps one of these for its lifetime.
#[derive(Default)]
pub struct Toolbars {
    /// The open flyout: its button's command id and the button rectangle it hangs from.
    flyout: Option<(&'static str, Rect)>,
    /// Where the open flyout was last drawn, so a press outside it closes it.
    flyout_rect: Option<Rect>,
    /// The button held down, and when the press began.
    press: Option<(&'static str, f64)>,
    /// The flyout opened from a hold, so the release does not also run the button.
    held: bool,
}

/// The engine id or file action a button id stands for. File and edit ids keep the menus' behaviour.
fn pick_for(id: &'static str) -> Pick {
    match id {
        "file.new" => Pick::File(FileAction::New),
        "doc.open" => Pick::File(FileAction::Open),
        "file.save" => Pick::File(FileAction::Save),
        "edit.undo" => Pick::File(FileAction::Undo),
        "edit.redo" => Pick::File(FileAction::Redo),
        _ => Pick::Start(id),
    }
}

impl Toolbars {
    /// The standard toolbar row under the menu bar. Returns the command a click asked for.
    pub fn standard(&mut self, ui: &mut Ui, cad: &SolveApp) -> Option<Pick> {
        if cad.nodes.open {
            return None;
        }
        self.close_outside(ui.ctx());
        let tokens = Tokens::get();
        let mut picked = None;
        Container::new(ui.ctx(), Surface::ApplicationBar)
            .edge("arclet_standard", Edge::Top)
            .frame(Container::new(ui.ctx(), Surface::ApplicationBar).edge_frame(tokens.panel).inner_margin(egui::Margin::symmetric(12, 4)))
            .show(ui, |ui| {
                // All toolbar commands remain reachable on narrow or high-DPI windows.
                // Do not wrap the row: wrapping would unexpectedly move the viewport.
                egui::ScrollArea::horizontal()
                    .id_salt("arclet_standard_scroll")
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = chrome::ITEM_GAP;
                            for (index, row) in STANDARD.iter().enumerate() {
                                if index > 0 {
                                    ui.add_space(chrome::SECTION_GAP);
                                    ui.separator();
                                    ui.add_space(chrome::SECTION_GAP);
                                }
                                for group in row.iter() {
                                    if let Some(pick) = self.button(ui, cad, group, vec2(STANDARD_BUTTON, STANDARD_BUTTON)) {
                                        picked = Some(pick);
                                    }
                                }
                            }
                        });
                    });
            });
        picked
    }

    /// The narrow main toolbar docked on the left. Rules separate the command groups.
    pub fn main(&mut self, ui: &mut Ui, cad: &SolveApp) -> Option<Pick> {
        if cad.nodes.open {
            return None;
        }
        self.close_outside(ui.ctx());
        let tokens = Tokens::get();
        let mut picked = None;
        Container::new(ui.ctx(), Surface::ApplicationBar)
            .edge("arclet_main", Edge::Left)
            .exact_size(MAIN_WIDTH)
            .frame(Container::new(ui.ctx(), Surface::ApplicationBar).edge_frame(tokens.panel).inner_margin(egui::Margin::symmetric(4, 6)))
            .show(ui, |ui| {
                // Scrolls instead of clipping when the window is short (minimum 960x600).
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.spacing_mut().item_spacing.y = chrome::ITEM_GAP;
                        for (index, row) in MAIN.iter().enumerate() {
                            if index > 0 {
                                ui.add_space(chrome::SECTION_GAP);
                                ui.separator();
                                ui.add_space(chrome::SECTION_GAP);
                            }
                            for group in row.iter() {
                                if let Some(pick) = self.button(ui, cad, group, vec2(MAIN_BUTTON, MAIN_BUTTON)) {
                                    picked = Some(pick);
                                }
                            }
                        }
                    })
                });
            });
        picked
    }

    /// One button: paints it, handles click, right click and press-and-hold, and draws its flyout.
    fn button(&mut self, ui: &mut Ui, cad: &SolveApp, group: &Group, size: egui::Vec2) -> Option<Pick> {
        let tokens = Tokens::get();
        let (id, label) = (group.id, group.label);
        let enabled = match pick_for(id) {
            Pick::File(action) => action.enabled(cad),
            Pick::Start(_) => true,
        };
        let has_flyout = group.more.len() >= 2;
        let open = self.flyout.is_some_and(|(key, _)| key == id);
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
        if enabled && (open || response.is_pointer_button_down_on()) {
            ui.painter().rect_filled(rect, tokens.radius_sm, tokens.accent_soft);
            ui.painter().rect_stroke(rect, tokens.radius_sm, Stroke::new(1.0, tokens.accent), egui::StrokeKind::Inside);
        } else if response.hovered() && enabled {
            ui.painter().rect_filled(rect, tokens.radius_sm, tokens.hover);
        }
        if response.has_focus() {
            ui.painter().rect_stroke(rect, tokens.radius_sm, Stroke::new(1.0, tokens.accent), egui::StrokeKind::Inside);
        }
        let ink = if !enabled { tokens.text_dim } else if open { tokens.accent } else { tokens.text };
        paint(ui.painter(), Rect::from_center_size(rect.center(), vec2(ICON, ICON)), group.glyph, ink);
        if has_flyout {
            let corner = rect.right_bottom() - vec2(3.0, 3.0);
            ui.painter().add(Shape::convex_polygon(vec![corner, corner - vec2(0.0, 6.0), corner - vec2(6.0, 0.0)], tokens.text_dim, Stroke::NONE));
        }
        response.clone().on_hover_ui(|ui| {
            ui.label(tr(label));
            ui.weak(id);
            if !enabled {
                ui.label(tr("arclet.toolbar.unavailable_context"));
            } else if has_flyout {
                ui.label(tr("arclet.toolbar.more_hint"));
            }
        });
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, tr(label)));
        solidcraft_ui_egui::scenario::publish_handle(&format!("arclet:tool:{id}"), rect.center());

        let mut pick = None;
        if response.clicked() && enabled {
            if self.held {
                // The hold already opened the flyout, so this release only ends the gesture.
                self.held = false;
            } else {
                pick = Some(pick_for(id));
                self.flyout = None;
                self.flyout_rect = None;
            }
        }
        if has_flyout {
            // Right click, Shift+F10 or Alt+Down open the same keyboard-accessible
            // menu. The primary click always runs the command itself.
            let keyboard_menu = response.has_focus()
                && ui.input(|input| {
                    (input.modifiers.shift && input.key_pressed(egui::Key::F10))
                        || (input.modifiers.alt && input.key_pressed(egui::Key::ArrowDown))
                });
            if response.secondary_clicked() || keyboard_menu {
                self.flyout = if open { None } else { Some((id, rect)) };
                self.flyout_rect = None;
            }
            if response.is_pointer_button_down_on() {
                let now = ui.input(|input| input.time);
                let started = match self.press {
                    Some((key, began)) if key == id => began,
                    _ => {
                        self.press = Some((id, now));
                        now
                    }
                };
                if !self.held && !open && now - started >= HOLD_SECONDS {
                    self.flyout = Some((id, rect));
                    self.flyout_rect = None;
                    self.held = true;
                }
                ui.ctx().request_repaint_after(Duration::from_millis(50));
            } else if self.press.is_some_and(|(key, _)| key == id) {
                self.press = None;
                self.held = false;
            }
        }
        if self.flyout.is_some_and(|(key, _)| key == id)
            && let Some(chosen) = self.draw_flyout(ui.ctx(), group, rect)
        {
            pick = Some(chosen);
        }
        pick
    }

    /// The popup listing a button's command and its related commands. Returns a pick when one is clicked.
    fn draw_flyout(&mut self, ctx: &egui::Context, group: &Group, anchor: Rect) -> Option<Pick> {
        let mut chosen = None;
        let viewport = ctx.viewport_rect();
        let max_width = (viewport.width() - 2.0 * chrome::FLOATING_MARGIN).max(1.0);
        let max_height = (viewport.height() - 2.0 * chrome::FLOATING_MARGIN).max(1.0);
        let width = chrome::FLYOUT_WIDTH.min(max_width);
        let desired_height = (group.more.len() + 1) as f32
            * (Tokens::get().control_height + chrome::FLYOUT_ROW_PADDING)
            + Tokens::get().space_12;
        let size = vec2(width, desired_height.min(max_height));
        let origin = chrome::flyout_origin(viewport, anchor, size);
        let area = Container::new(ctx, Surface::Popup).area(Id::new(("arclet_flyout", group.id))).fixed_pos(origin);
        let shown = area.show(ctx, |ui| {
            Container::popup_frame(ui.style()).show(ui, |ui| {
                ui.set_min_width(width);
                egui::ScrollArea::vertical()
                    .id_salt(("arclet_flyout_scroll", group.id))
                    .max_height(max_height)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            for &(label, id) in std::iter::once(&(group.label, group.id)).chain(group.more.iter()) {
                                if ui_kit::button(ui, tr(label), ui_kit::Button::Ghost).on_hover_text(id).clicked() {
                                    chosen = Some(pick_for(id));
                                }
                            }
                        });
                    });
            });
        });
        self.flyout_rect = Some(shown.response.rect);
        if chosen.is_some() {
            self.flyout = None;
            self.flyout_rect = None;
        }
        chosen
    }

    /// Closes the flyout on Escape or on a press outside both the button and the flyout.
    fn close_outside(&mut self, ctx: &egui::Context) {
        let Some((_, button)) = self.flyout else {
            return;
        };
        let (pressed, at, escape) =
            ctx.input(|input| (input.pointer.any_pressed(), input.pointer.interact_pos(), input.key_pressed(egui::Key::Escape)));
        let outside = pressed && at.is_some_and(|pos| !button.contains(pos) && !self.flyout_rect.is_some_and(|rect| rect.contains(pos)));
        if escape || outside {
            self.flyout = None;
            self.flyout_rect = None;
        }
    }
}

/// Draws one icon inside `rect` from points given in unit-square coordinates.
fn paint(painter: &Painter, rect: Rect, glyph: Glyph, ink: Color32) {
    let stroke = Stroke::new(1.5, ink);
    let flip = glyph == Glyph::Redo;
    let at = |x: f32, y: f32| {
        let x = if flip { 1.0 - x } else { x };
        pos2(rect.left() + rect.width() * x, rect.top() + rect.height() * y)
    };
    let line = |points: &[(f32, f32)]| {
        painter.add(Shape::line(points.iter().map(|&(x, y)| at(x, y)).collect(), stroke));
    };
    let closed = |points: &[(f32, f32)]| {
        painter.add(Shape::closed_line(points.iter().map(|&(x, y)| at(x, y)).collect(), stroke));
    };
    let dot = |x: f32, y: f32| {
        painter.circle_filled(at(x, y), 0.07 * rect.width(), ink);
    };
    let ring = |cx: f32, cy: f32, rx: f32, ry: f32| closed(&arc(cx, cy, rx, ry, 0.0, TAU, 36));

    match glyph {
        Glyph::NewPage => {
            closed(&[(0.25, 0.1), (0.6, 0.1), (0.78, 0.3), (0.78, 0.9), (0.25, 0.9)]);
            line(&[(0.6, 0.1), (0.6, 0.3), (0.78, 0.3)]);
        }
        Glyph::Folder => closed(&[(0.1, 0.3), (0.4, 0.3), (0.5, 0.4), (0.9, 0.4), (0.9, 0.85), (0.1, 0.85)]),
        Glyph::Disk => {
            closed(&rect_pts(0.12, 0.12, 0.88, 0.88));
            line(&[(0.3, 0.12), (0.3, 0.4), (0.7, 0.4), (0.7, 0.12)]);
            line(&[(0.3, 0.88), (0.3, 0.6), (0.7, 0.6), (0.7, 0.88)]);
        }
        Glyph::Undo | Glyph::Redo => {
            // A curved arrow over the top, ending with a head that points down at the left end.
            line(&arc(0.5, 0.55, 0.32, 0.28, 0.0, -PI, 20));
            line(&chevron((0.18, 0.55), (0.0, 1.0), 0.12));
        }
        Glyph::Repeat => {
            let (a0, a1) = (-1.2_f32, 4.3_f32);
            let arc_points = arc(0.5, 0.5, 0.34, 0.34, a0, a1, 24);
            line(&arc_points);
            let (ex, ey) = (0.5 + 0.34 * a1.cos(), 0.5 + 0.34 * a1.sin());
            line(&chevron((ex, ey), (-a1.sin(), a1.cos()), 0.14));
        }
        Glyph::Pointer => closed(&[(0.3, 0.12), (0.3, 0.8), (0.45, 0.66), (0.6, 0.9), (0.7, 0.85), (0.56, 0.6), (0.78, 0.6)]),
        Glyph::Trash => {
            line(&[(0.2, 0.28), (0.8, 0.28)]);
            line(&[(0.4, 0.28), (0.4, 0.15), (0.6, 0.15), (0.6, 0.28)]);
            line(&[(0.28, 0.28), (0.33, 0.88), (0.67, 0.88), (0.72, 0.28)]);
            line(&[(0.44, 0.4), (0.44, 0.76)]);
            line(&[(0.56, 0.4), (0.56, 0.76)]);
        }
        Glyph::Move => {
            line(&[(0.5, 0.1), (0.5, 0.9)]);
            line(&[(0.1, 0.5), (0.9, 0.5)]);
            line(&chevron((0.5, 0.1), (0.0, -1.0), 0.14));
            line(&chevron((0.5, 0.9), (0.0, 1.0), 0.14));
            line(&chevron((0.1, 0.5), (-1.0, 0.0), 0.14));
            line(&chevron((0.9, 0.5), (1.0, 0.0), 0.14));
        }
        Glyph::Scale => {
            closed(&rect_pts(0.12, 0.4, 0.6, 0.88));
            line(&[(0.5, 0.5), (0.88, 0.12)]);
            line(&chevron((0.88, 0.12), (0.707, -0.707), 0.2));
        }
        Glyph::ZoomExtents => {
            line(&[(0.12, 0.3), (0.12, 0.12), (0.3, 0.12)]);
            line(&[(0.7, 0.12), (0.88, 0.12), (0.88, 0.3)]);
            line(&[(0.88, 0.7), (0.88, 0.88), (0.7, 0.88)]);
            line(&[(0.3, 0.88), (0.12, 0.88), (0.12, 0.7)]);
            closed(&rect_pts(0.4, 0.4, 0.6, 0.6));
        }
        Glyph::Iso => {
            closed(&[(0.5, 0.1), (0.86, 0.3), (0.86, 0.7), (0.5, 0.9), (0.14, 0.7), (0.14, 0.3)]);
            line(&[(0.14, 0.3), (0.5, 0.5), (0.86, 0.3)]);
            line(&[(0.5, 0.5), (0.5, 0.9)]);
        }
        Glyph::Layout => {
            closed(&rect_pts(0.12, 0.12, 0.46, 0.46));
            closed(&rect_pts(0.54, 0.12, 0.88, 0.46));
            closed(&rect_pts(0.12, 0.54, 0.46, 0.88));
            closed(&rect_pts(0.54, 0.54, 0.88, 0.88));
        }
        Glyph::Line => {
            line(&[(0.15, 0.85), (0.85, 0.15)]);
            dot(0.15, 0.85);
            dot(0.85, 0.15);
        }
        Glyph::Circle => ring(0.5, 0.5, 0.36, 0.36),
        Glyph::Arc => {
            line(&arc(0.5, 0.6, 0.36, 0.36, -PI, 0.0, 24));
            dot(0.14, 0.6);
            dot(0.86, 0.6);
        }
        Glyph::Rectangle => closed(&rect_pts(0.15, 0.25, 0.85, 0.75)),
        Glyph::Spline => {
            line(&bezier([(0.12, 0.75), (0.3, 0.05), (0.7, 0.95), (0.88, 0.25)], 24));
            dot(0.12, 0.75);
            dot(0.88, 0.25);
        }
        Glyph::Extrude => {
            closed(&rect_pts(0.12, 0.4, 0.6, 0.88));
            closed(&rect_pts(0.4, 0.12, 0.88, 0.6));
            line(&[(0.12, 0.4), (0.4, 0.12)]);
            line(&[(0.6, 0.4), (0.88, 0.12)]);
            line(&[(0.6, 0.88), (0.88, 0.6)]);
            line(&[(0.12, 0.88), (0.4, 0.6)]);
        }
        Glyph::Loft => {
            closed(&rect_pts(0.08, 0.6, 0.42, 0.92));
            closed(&rect_pts(0.56, 0.08, 0.92, 0.4));
            line(&[(0.08, 0.6), (0.56, 0.08)]);
            line(&[(0.42, 0.92), (0.92, 0.4)]);
        }
        Glyph::Sweep => {
            ring(0.2, 0.78, 0.12, 0.12);
            line(&bezier([(0.26, 0.78), (0.6, 0.9), (0.9, 0.6), (0.84, 0.18)], 24));
        }
        Glyph::Revolve => {
            line(&[(0.2, 0.1), (0.2, 0.9)]);
            closed(&rect_pts(0.4, 0.25, 0.8, 0.75));
            line(&arc(0.2, 0.5, 0.12, 0.4, -PI / 2.0, PI / 2.0, 12));
        }
        Glyph::Boolean => {
            ring(0.38, 0.5, 0.26, 0.26);
            ring(0.62, 0.5, 0.26, 0.26);
        }
        Glyph::Fillet => {
            line(&[(0.2, 0.12), (0.2, 0.55)]);
            line(&arc(0.55, 0.55, 0.35, 0.35, PI, PI / 2.0, 12));
            line(&[(0.55, 0.9), (0.88, 0.9)]);
        }
        Glyph::Trim => {
            line(&[(0.12, 0.5), (0.42, 0.5)]);
            line(&[(0.6, 0.5), (0.88, 0.5)]);
            line(&[(0.6, 0.1), (0.4, 0.9)]);
        }
        Glyph::Offset => {
            closed(&rect_pts(0.25, 0.3, 0.7, 0.72));
            closed(&rect_pts(0.12, 0.16, 0.85, 0.86));
        }
        Glyph::Mirror => {
            for step in 0..4 {
                let top = 0.08 + step as f32 * 0.22;
                line(&[(0.5, top), (0.5, top + 0.14)]);
            }
            closed(&[(0.12, 0.2), (0.12, 0.75), (0.42, 0.75)]);
            closed(&[(0.88, 0.2), (0.88, 0.75), (0.58, 0.75)]);
        }
        Glyph::Measure => {
            // A ruler band with tick marks along its length.
            let (a, b) = ((0.12_f32, 0.72_f32), (0.88_f32, 0.28_f32));
            let (dx, dy) = (b.0 - a.0, b.1 - a.1);
            let length = (dx * dx + dy * dy).sqrt();
            let normal = (-dy / length * 0.14, dx / length * 0.14);
            closed(&[a, b, (b.0 + normal.0, b.1 + normal.1), (a.0 + normal.0, a.1 + normal.1)]);
            for t in [0.2_f32, 0.4, 0.6, 0.8] {
                let p = (a.0 + dx * t, a.1 + dy * t);
                line(&[p, (p.0 + normal.0 * 0.6, p.1 + normal.1 * 0.6)]);
            }
        }
        Glyph::Zebra => {
            ring(0.5, 0.5, 0.36, 0.36);
            for band in [0.36_f32, 0.5, 0.64] {
                let wave: Vec<(f32, f32)> = (0..=12)
                    .map(|i| {
                        let t = i as f32 / 12.0;
                        (0.2 + 0.6 * t, band + 0.05 * (t * TAU * 1.5).sin())
                    })
                    .collect();
                line(&wave);
            }
        }
        Glyph::Curvature => {
            let curve: Vec<(f32, f32)> = (0..=20)
                .map(|i| {
                    let t = i as f32 / 20.0;
                    (0.12 + 0.76 * t, 0.78 - 0.4 * (PI * t).sin())
                })
                .collect();
            line(&curve);
            for t in [0.15_f32, 0.3, 0.5, 0.7, 0.85] {
                let x = 0.12 + 0.76 * t;
                let y = 0.78 - 0.4 * (PI * t).sin();
                line(&[(x, y), (x, y - (0.08 + 0.2 * (1.0 - (2.0 * t - 1.0).abs())))]);
            }
        }
    }
}

fn rect_pts(x0: f32, y0: f32, x1: f32, y1: f32) -> [(f32, f32); 4] {
    [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
}

/// Points along an elliptical arc from angle `a0` to `a1` in radians, screen coordinates.
fn arc(cx: f32, cy: f32, rx: f32, ry: f32, a0: f32, a1: f32, steps: u32) -> Vec<(f32, f32)> {
    (0..=steps)
        .map(|i| {
            let a = a0 + (a1 - a0) * (i as f32 / steps as f32);
            (cx + rx * a.cos(), cy + ry * a.sin())
        })
        .collect()
}

/// A cubic Bezier sampled into `steps` segments.
fn bezier(p: [(f32, f32); 4], steps: u32) -> Vec<(f32, f32)> {
    (0..=steps)
        .map(|i| {
            let t = i as f32 / steps as f32;
            let u = 1.0 - t;
            let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
            (a * p[0].0 + b * p[1].0 + c * p[2].0 + d * p[3].0, a * p[0].1 + b * p[1].1 + c * p[2].1 + d * p[3].1)
        })
        .collect()
}

/// Two short arms behind `tip`, opening against the travel direction `dir`.
fn chevron(tip: (f32, f32), dir: (f32, f32), len: f32) -> [(f32, f32); 3] {
    let back = (tip.0 - dir.0 * len, tip.1 - dir.1 * len);
    let side = (-dir.1 * len * 0.6, dir.0 * len * 0.6);
    [(back.0 + side.0, back.1 + side.1), tip, (back.0 - side.0, back.1 - side.1)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_toolbar_button_starts_an_engine_command() {
        for row in STANDARD.iter().chain(MAIN.iter()) {
            for group in row.iter() {
                let ids = std::iter::once(group.id).chain(group.more.iter().map(|&(_, id)| id));
                for id in ids {
                    assert!(solidcraft_engine::find_command(id).is_some(), "{id} is not an engine command");
                }
            }
        }
    }


    #[test]
    fn toolbar_metrics_leave_room_for_pointer_targets() {
        assert!(STANDARD_BUTTON >= 30.0);
        assert!(MAIN_BUTTON >= 34.0);
        assert!(MAIN_WIDTH >= MAIN_BUTTON + 2.0 * chrome::SECTION_GAP);
        assert!(ICON < STANDARD_BUTTON && ICON < MAIN_BUTTON);
        assert!(chrome::FLYOUT_WIDTH >= 200.0);
    }

    #[test]
    fn a_flyout_has_three_distinct_commands_and_no_repeats() {
        for row in MAIN.iter().chain(STANDARD.iter()) {
            for group in row.iter() {
                if group.more.is_empty() {
                    continue;
                }
                assert!(group.more.len() >= 2, "{} has a flyout of fewer than three commands", group.id);
                let mut ids: Vec<&str> = std::iter::once(group.id).chain(group.more.iter().map(|&(_, id)| id)).collect();
                ids.sort_unstable();
                let before = ids.len();
                ids.dedup();
                assert_eq!(before, ids.len(), "{} repeats a command in its flyout", group.id);
            }
        }
    }
}
