//! Resolution-independent game pieces; all colors come from the active palette.
use eframe::egui::{self, Painter, Rect, Stroke, StrokeKind, Vec2};
use sokobanchy::theme::Theme;

use crate::blend;

pub fn goal(p: &Painter, tile: Rect, t: &Theme) {
    // Hollow corner brackets stay visible when the agent stands on an empty goal.
    let s = tile.width();
    let slot = tile.shrink(s * 0.08);
    let floor = blend(t.background, t.panel, 0.6);
    let line = Stroke::new((s * 0.03).max(1.0), blend(floor, t.muted, 0.7));
    for (corner, dx, dy) in [
        (slot.left_top(), 1.0, 1.0),
        (slot.right_top(), -1.0, 1.0),
        (slot.left_bottom(), 1.0, -1.0),
        (slot.right_bottom(), -1.0, -1.0),
    ] {
        p.line_segment([corner, corner + Vec2::new(dx, 0.0) * s * 0.17], line);
        p.line_segment([corner, corner + Vec2::new(0.0, dy) * s * 0.17], line);
    }
}

fn completed_library(p: &Painter, tile: Rect, t: &Theme) {
    let s = tile.width();
    let floor = blend(t.background, t.panel, 0.6);
    p.rect_stroke(
        tile.shrink(2.0),
        2,
        Stroke::new(1.0_f32, blend(floor, t.green, 0.35)),
        StrokeKind::Inside,
    );
    let mark = Rect::from_center_size(tile.center(), Vec2::splat(s * 0.68));
    p.rect_filled(mark, 0, t.green);
    // Exact geometry of Omarchy's icon.png, expressed on a 15×15 grid.
    // Copyright (c) David Heinemeier Hansson, MIT; see assets/OMARCHY-LICENSE.
    for [x, y, w, h] in [
        [1, 1, 6, 1],
        [8, 1, 6, 1],
        [1, 2, 1, 1],
        [8, 2, 3, 1],
        [13, 2, 1, 1],
        [1, 3, 1, 4],
        [3, 3, 9, 4],
        [13, 3, 1, 4],
        [3, 7, 9, 1],
        [13, 7, 1, 1],
        [1, 8, 1, 4],
        [3, 8, 9, 4],
        [13, 8, 1, 4],
        [1, 12, 1, 1],
        [13, 12, 1, 1],
        [1, 13, 6, 1],
        [8, 13, 6, 1],
        [8, 14, 1, 1],
    ] {
        p.rect_filled(
            Rect::from_min_size(
                mark.min + Vec2::new(x as f32, y as f32) * mark.width() / 15.0,
                Vec2::new(w as f32, h as f32) * mark.width() / 15.0,
            ),
            0,
            floor,
        );
    }
}

pub fn library(p: &Painter, tile: Rect, t: &Theme, on_goal: bool) {
    if on_goal {
        completed_library(p, tile, t);
        return;
    }
    let s = tile.width();
    let c = t.yellow;
    let cover = tile.shrink(s * 0.14).translate(Vec2::new(0.0, -s * 0.025));
    let line = Stroke::new((s * 0.035).max(1.1), c);
    let pages = cover.translate(Vec2::new(s * 0.045, s * 0.07));
    p.rect_filled(pages, 2, t.panel);
    p.rect_stroke(pages, 2, line, StrokeKind::Inside);
    p.rect_filled(cover, 2, blend(t.background, c, 0.18));
    p.rect_stroke(cover, 2, line, StrokeKind::Inside);
    let spine = cover.left() + s * 0.11;
    p.line_segment(
        [
            egui::pos2(spine, cover.top()),
            egui::pos2(spine, cover.bottom()),
        ],
        line,
    );
    p.text(
        cover.center() + Vec2::new(s * 0.045, -s * 0.02),
        egui::Align2::CENTER_CENTER,
        "{}",
        egui::FontId::monospace(s * 0.34),
        c,
    );
}

pub fn agent(p: &Painter, tile: Rect, t: &Theme, smiling: bool) {
    let s = tile.width();
    let center = tile.center();
    let line = Stroke::new((s * 0.035).max(1.0), t.accent);
    let point = |x, y| center + Vec2::new(x, y) * s;
    p.line_segment([point(0.0, -0.24), point(0.0, -0.37)], line);
    p.circle_filled(point(0.0, -0.38), s * 0.045, t.accent);
    for x in [-0.34, 0.34] {
        p.rect_filled(
            Rect::from_center_size(point(x, -0.01), Vec2::new(0.08, 0.20) * s),
            2,
            t.accent,
        );
    }
    let head = Rect::from_center_size(point(0.0, -0.015), Vec2::new(0.62, 0.48) * s);
    p.rect_filled(head.translate(Vec2::new(0.0, 2.0)), 5, t.panel);
    p.rect_filled(head, 5, t.accent);
    p.rect_filled(head.shrink(s * 0.07), 3, t.background);
    for x in [-0.115, 0.115] {
        p.rect_filled(
            Rect::from_center_size(point(x, -0.05), Vec2::new(0.065, 0.09) * s),
            1,
            t.accent,
        );
    }
    if smiling {
        p.add(egui::Shape::line(
            vec![
                point(-0.12, 0.035),
                point(-0.08, 0.10),
                point(0.0, 0.125),
                point(0.08, 0.10),
                point(0.12, 0.035),
            ],
            line,
        ));
    } else {
        p.line_segment([point(-0.05, 0.08), point(0.05, 0.08)], line);
    }
    p.rect_filled(
        Rect::from_center_size(point(0.0, 0.28), Vec2::new(0.34, 0.08) * s),
        2,
        t.accent,
    );
}

pub fn legend(ui: &mut egui::Ui, t: &Theme) {
    ui.horizontal_wrapped(|ui| {
        for (kind, label) in [
            (0, "AI agent"),
            (1, "Library"),
            (2, "Goal"),
            (3, "Delivered"),
        ] {
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(34.0), egui::Sense::hover());
            match kind {
                0 => agent(ui.painter(), rect, t, false),
                1 => library(ui.painter(), rect, t, false),
                2 => goal(ui.painter(), rect, t),
                _ => library(ui.painter(), rect, t, true),
            }
            ui.label(egui::RichText::new(label).size(11.0).color(t.foreground));
        }
    });
}
