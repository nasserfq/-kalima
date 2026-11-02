//! The right-to-left interface for كلمة: every frame is laid out as usual, then drawn mirrored
//! (ribbon, menus, panes, dialogs and the status bar read from the right), with pointer input
//! mirrored back so clicks land where they are drawn. Text, pictures and icons' textures are not
//! flipped. The document area (rulers and pages, [`keep`]) is moved as a block instead of being
//! mirrored, because the page is already laid out right to left by the document itself.
//!
//! Every window is mirrored, each with its own state: documents opened in further native windows
//! are egui viewports of their own. Dialogs and other windows ([`overlay`]), with the menus and
//! tooltips that open on them, are always mirrored as a whole, even over the document area. shortcut: accessibility bounds stay unmirrored; per-widget
//! layout would fix that.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use egui::epaint::ClippedShape;
use egui::layers::ShapeIdx;
use egui::{CornerRadius, Event, LayerId, Order, Pos2, Rect, Shape, TextureId, ViewportId, pos2, vec2};

/// One window's mirroring.
#[derive(Clone, Default)]
struct Window {
    /// `min.x + max.x` of the window: x mirrors to `span − x`.
    span: f32,
    /// The document area drawn this frame, and the one drawn last frame (for input).
    keep: Option<Rect>,
    last_keep: Option<Rect>,
    /// Windows registered this frame with [`overlay`].
    overlays: Vec<LayerId>,
    /// Where last frame's overlays, their menus and tooltips were drawn (for input).
    last_overlays: Vec<Rect>,
}

thread_local! {
    static WINDOWS: RefCell<HashMap<ViewportId, Window>> = RefCell::new(HashMap::new());
}

fn with<R>(id: ViewportId, f: impl FnOnce(&mut Window) -> R) -> R {
    WINDOWS.with_borrow_mut(|w| f(w.entry(id).or_default()))
}

/// Is the interface mirrored? Whenever the interface is Arabic (outside tests).
pub fn active() -> bool {
    !cfg!(test) && crate::i18n::current().code() == crate::i18n::ONLY_LANGUAGE
}

/// The document area of `ctx`'s window this frame (rulers and pages): moved, not mirrored.
pub fn keep(ctx: &egui::Context, rect: Rect) {
    with(ctx.viewport_id(), |w| w.keep = Some(rect));
}

/// A dialog or other window shown this frame in `ctx`'s window: mirrored as a whole, with any
/// menu or tooltip that touches it, never moved with the document area under it.
pub fn overlay(ctx: &egui::Context, layer: LayerId) {
    with(ctx.viewport_id(), |w| w.overlays.push(layer));
}

fn mirror_rect(s: f32, r: Rect) -> Rect {
    Rect::from_min_max(pos2(s - r.max.x, r.min.y), pos2(s - r.min.x, r.max.y))
}

/// How far the document area moves: from where it is laid out to its mirrored place.
fn keep_dx(s: f32, keep: Rect) -> f32 {
    (s - keep.max.x) - keep.min.x
}

/// A point as the user sees it, back to where the layout put it.
fn to_layout(p: Pos2, s: f32, keep: Option<Rect>, overlays: &[Rect]) -> Pos2 {
    if overlays.iter().any(|r| r.contains(p)) {
        return pos2(s - p.x, p.y);
    }
    match keep {
        Some(k) if mirror_rect(s, k).contains(p) => pos2(p.x - keep_dx(s, k), p.y),
        _ => pos2(s - p.x, p.y),
    }
}

/// A rect in layout space, to where it is drawn.
fn to_screen(r: Rect, s: f32, keep: Option<Rect>) -> Rect {
    match keep {
        Some(k) if k.expand(1.0).contains_rect(r) => r.translate(vec2(keep_dx(s, k), 0.0)),
        _ => mirror_rect(s, r),
    }
}

/// Mirror a window's pointer input for this frame (call from an input hook, before egui reads it).
pub fn input(raw: &mut egui::RawInput) {
    if !active() {
        return;
    }
    let (s, keep, overlays) = with(raw.viewport_id, |w| {
        if let Some(r) = raw.screen_rect {
            w.span = r.min.x + r.max.x;
        }
        (w.span, w.last_keep, w.last_overlays.clone())
    });
    if s <= 0.0 {
        return;
    }
    for e in &mut raw.events {
        match e {
            Event::PointerMoved(p) => *p = to_layout(*p, s, keep, &overlays),
            Event::PointerButton { pos, .. } | Event::Touch { pos, .. } => *pos = to_layout(*pos, s, keep, &overlays),
            Event::MouseMoved(d) => d.x = -d.x,
            _ => {}
        }
    }
}

/// Mirror everything painted in `ctx`'s window this frame (call after the window's `ui`).
pub fn output(ctx: &egui::Context) {
    let s = ctx.input(|i| {
        let r = i.viewport_rect();
        r.min.x + r.max.x
    });
    let (keep, overlays) = with(ctx.viewport_id(), |w| {
        let keep = w.keep.take();
        w.last_keep = keep;
        w.last_overlays.clear();
        if s > 0.0 {
            w.span = s;
        }
        (keep, std::mem::take(&mut w.overlays))
    });
    if !active() || s <= 0.0 {
        return;
    }
    let mut ids: Vec<LayerId> = ctx.memory(|m| m.layer_ids().collect());
    if !ids.contains(&LayerId::background()) {
        ids.push(LayerId::background());
    }
    // The overlays, and the menus and tooltips on them (a combo box's list may hang past the
    // dialog's edge, so it only has to touch it): mirrored whole, whatever is under them.
    let rect = |id: LayerId| ctx.memory(|m| m.area_rect(id.id));
    let dialogs: Vec<Rect> = overlays.iter().filter_map(|&id| rect(id)).collect();
    let whole: Vec<(LayerId, Rect)> = ids
        .iter()
        .filter_map(|&id| Some((id, rect(id)?)))
        .filter(|&(id, r)| {
            overlays.contains(&id) || (matches!(id.order, Order::Foreground | Order::Tooltip) && dialogs.iter().any(|d| d.expand(8.0).intersects(r)))
        })
        .collect();
    with(ctx.viewport_id(), |w| w.last_overlays = whole.iter().map(|&(_, r)| mirror_rect(s, r)).collect());
    ctx.graphics_mut(|g| {
        for id in ids {
            let keep = if whole.iter().any(|&(w, _)| w == id) { None } else { keep };
            if let Some(list) = g.get_mut(id) {
                for i in 0..list.next_idx().0 {
                    list.mutate_shape(ShapeIdx(i), |c| clipped(c, s, keep));
                }
            }
        }
    });
    ctx.output_mut(|o| {
        if let Some(ime) = &mut o.ime {
            // A text field in a dialog is mirrored with its dialog.
            let keep = if whole.iter().any(|&(_, r)| r.contains_rect(ime.rect)) { None } else { keep };
            ime.rect = to_screen(ime.rect, s, keep);
            ime.cursor_rect = to_screen(ime.cursor_rect, s, keep);
        }
    });
}

fn clipped(c: &mut ClippedShape, s: f32, keep: Option<Rect>) {
    if let Some(k) = keep
        && k.expand(1.0).contains_rect(c.clip_rect)
    {
        let d = vec2(keep_dx(s, k), 0.0);
        c.clip_rect = c.clip_rect.translate(d);
        c.shape.translate(d);
        return;
    }
    c.clip_rect = mirror_rect(s, c.clip_rect);
    flip(&mut c.shape, s);
    // A transform applied after tessellation (a popup scaling in) maps the mirrored shape to the
    // mirror of where it would have gone.
    let t = &mut c.transform_after_tessellation;
    t.translation.x = s - t.translation.x - t.scaling * s;
}

fn flip(shape: &mut Shape, s: f32) {
    let fx = |p: &mut Pos2| p.x = s - p.x;
    match shape {
        Shape::Noop => {}
        Shape::Vec(v) => v.iter_mut().for_each(|x| flip(x, s)),
        Shape::Circle(c) => fx(&mut c.center),
        Shape::Ellipse(e) => {
            fx(&mut e.center);
            e.angle = -e.angle;
        }
        Shape::LineSegment { points, .. } => points.iter_mut().for_each(fx),
        Shape::Path(p) => p.points.iter_mut().for_each(fx),
        Shape::Band(b) => {
            b.points.iter_mut().for_each(|p| p.x = s - p.x);
            b.points.reverse(); // kept sorted by x
        }
        Shape::Rect(r) => {
            r.rect = mirror_rect(s, r.rect);
            let c = r.corner_radius;
            r.corner_radius = CornerRadius { nw: c.ne, ne: c.nw, sw: c.se, se: c.sw };
            r.angle = -r.angle;
            if let Some(b) = &mut r.brush {
                // Swapping the u range draws a picture the right way round in its mirrored rect.
                let b = Arc::make_mut(b);
                b.uv = Rect::from_min_max(pos2(b.uv.max.x, b.uv.min.y), pos2(b.uv.min.x, b.uv.max.y));
            }
        }
        Shape::Text(t) => {
            // Moved, never flipped: its galley keeps its own (bidi) order.
            let g = t.galley.rect;
            let min_x = s - (t.pos.x + g.max.x);
            t.pos.x = min_x - g.min.x;
        }
        Shape::Mesh(m) => {
            let m = Arc::make_mut(m);
            m.vertices.iter_mut().for_each(|v| v.pos.x = s - v.pos.x);
            if m.texture_id != TextureId::default() {
                // A picture: flip its texture coordinates back within the mesh's u range.
                let (lo, hi) = m.vertices.iter().fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), v| (lo.min(v.uv.x), hi.max(v.uv.x)));
                m.vertices.iter_mut().for_each(|v| v.uv.x = lo + hi - v.uv.x);
            }
        }
        Shape::QuadraticBezier(b) => b.points.iter_mut().for_each(fx),
        Shape::CubicBezier(b) => b.points.iter_mut().for_each(fx),
        Shape::Callback(c) => c.rect = mirror_rect(s, c.rect),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn points_round_trip_and_the_document_area_moves_as_a_block() {
        let s = 1000.0;
        let keep = Some(Rect::from_min_max(pos2(200.0, 50.0), pos2(900.0, 800.0)));
        // Outside the document area: mirrored.
        assert_eq!(to_layout(pos2(10.0, 5.0), s, keep, &[]), pos2(990.0, 5.0));
        // The area is drawn at 100..800: a point there maps back by the same shift.
        assert_eq!(to_layout(pos2(150.0, 100.0), s, keep, &[]), pos2(250.0, 100.0));
        let r = Rect::from_min_max(pos2(300.0, 60.0), pos2(310.0, 70.0));
        let drawn = to_screen(r, s, keep);
        assert_eq!(to_layout(drawn.center(), s, keep, &[]), r.center());
        // Over a dialog drawn above the document area: mirrored like the rest of the interface.
        let dialog = [Rect::from_min_max(pos2(120.0, 90.0), pos2(400.0, 300.0))];
        assert_eq!(to_layout(pos2(150.0, 100.0), s, keep, &dialog), pos2(850.0, 100.0));
    }

    #[test]
    fn text_moves_without_flipping() {
        let r = Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 20.0));
        let m = mirror_rect(1000.0, r);
        assert_eq!((m.min.x, m.max.x), (900.0, 1000.0));
    }
}
