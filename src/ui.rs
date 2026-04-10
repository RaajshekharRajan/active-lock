use tiny_skia::*;

const FIELD_W: f32 = 320.0;
const FIELD_H: f32 = 48.0;
const FIELD_RADIUS: f32 = 10.0;
const DOT_RADIUS: f32 = 6.0;
const DOT_SPACING: f32 = 20.0;
const LOCK_ICON_OFFSET_Y: f32 = 52.0;

pub fn render_lock_screen(
    width: u32,
    height: u32,
    scale: f32,
    password_len: usize,
    error: bool,
) -> Option<Pixmap> {
    let mut pixmap = Pixmap::new(width, height)?;
    pixmap.fill(Color::from_rgba8(0, 0, 0, 255));

    let s = scale;
    let cx = width as f32 / 2.0;
    let cy = height as f32 / 2.0;

    draw_lock_icon(&mut pixmap, cx, cy - LOCK_ICON_OFFSET_Y * s, s);

    let fw = FIELD_W * s;
    let fh = FIELD_H * s;
    let fx = cx - fw / 2.0;
    let fy = cy - fh / 2.0;
    let fr = FIELD_RADIUS * s;

    if let Some(path) = rounded_rect(fx, fy, fw, fh, fr) {
        let mut bg = Paint::default();
        bg.set_color_rgba8(22, 22, 22, 255);
        bg.anti_alias = true;
        pixmap.fill_path(&path, &bg, FillRule::Winding, Transform::identity(), None);

        let mut border = Paint::default();
        if error {
            border.set_color_rgba8(220, 50, 50, 255);
        } else {
            border.set_color_rgba8(58, 58, 58, 255);
        }
        border.anti_alias = true;
        let stroke = Stroke {
            width: 1.5 * s,
            ..Stroke::default()
        };
        pixmap.stroke_path(&path, &border, &stroke, Transform::identity(), None);
    }

    if password_len > 0 {
        let ds = DOT_SPACING * s;
        let dr = DOT_RADIUS * s;
        let total = password_len as f32 * ds;
        let start_x = cx - total / 2.0 + ds / 2.0;

        let mut dot_paint = Paint::default();
        dot_paint.set_color_rgba8(200, 200, 200, 255);
        dot_paint.anti_alias = true;

        for i in 0..password_len {
            let dot_cx = start_x + i as f32 * ds;
            let mut pb = PathBuilder::new();
            pb.push_circle(dot_cx, cy, dr);
            if let Some(circle) = pb.finish() {
                pixmap.fill_path(
                    &circle,
                    &dot_paint,
                    FillRule::Winding,
                    Transform::identity(),
                    None,
                );
            }
        }
    }

    Some(pixmap)
}

pub fn render_black_screen(width: u32, height: u32) -> Option<Pixmap> {
    let mut pixmap = Pixmap::new(width, height)?;
    pixmap.fill(Color::from_rgba8(0, 0, 0, 255));
    Some(pixmap)
}

fn draw_lock_icon(pixmap: &mut Pixmap, cx: f32, cy: f32, scale: f32) {
    let s = scale;
    let body_w = 26.0 * s;
    let body_h = 22.0 * s;
    let body_r = 4.0 * s;
    let shackle_w = 16.0 * s;
    let shackle_h = 14.0 * s;
    let shackle_thickness = 3.0 * s;

    let mut paint = Paint::default();
    paint.set_color_rgba8(70, 70, 70, 255);
    paint.anti_alias = true;

    // Shackle (U-shape on top of body)
    let shackle_top = cy - shackle_h;
    let shackle_left = cx - shackle_w / 2.0;
    let mut pb = PathBuilder::new();
    pb.move_to(shackle_left, cy);
    pb.line_to(shackle_left, shackle_top + body_r);
    pb.quad_to(shackle_left, shackle_top, cx, shackle_top);
    pb.quad_to(
        shackle_left + shackle_w,
        shackle_top,
        shackle_left + shackle_w,
        shackle_top + body_r,
    );
    pb.line_to(shackle_left + shackle_w, cy);
    if let Some(path) = pb.finish() {
        let stroke = Stroke {
            width: shackle_thickness,
            line_cap: LineCap::Round,
            ..Stroke::default()
        };
        pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    }

    // Body (rounded rect below the shackle)
    if let Some(path) = rounded_rect(cx - body_w / 2.0, cy, body_w, body_h, body_r) {
        pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
    }

    // Keyhole dot
    let mut kh = Paint::default();
    kh.set_color_rgba8(30, 30, 30, 255);
    kh.anti_alias = true;
    let mut pb = PathBuilder::new();
    pb.push_circle(cx, cy + body_h * 0.38, 3.0 * s);
    if let Some(circle) = pb.finish() {
        pixmap.fill_path(&circle, &kh, FillRule::Winding, Transform::identity(), None);
    }
}

fn rounded_rect(x: f32, y: f32, w: f32, h: f32, r: f32) -> Option<Path> {
    let mut pb = PathBuilder::new();
    pb.move_to(x + r, y);
    pb.line_to(x + w - r, y);
    pb.quad_to(x + w, y, x + w, y + r);
    pb.line_to(x + w, y + h - r);
    pb.quad_to(x + w, y + h, x + w - r, y + h);
    pb.line_to(x + r, y + h);
    pb.quad_to(x, y + h, x, y + h - r);
    pb.line_to(x, y + r);
    pb.quad_to(x, y, x + r, y);
    pb.close();
    pb.finish()
}
