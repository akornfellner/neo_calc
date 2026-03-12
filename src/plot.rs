use std::collections::HashMap;
use wasm_bindgen::JsCast;
use web_sys::CanvasRenderingContext2d;

use crate::eval::evaluate;

const CANVAS_W: f64 = 640.0;
const CANVAS_H: f64 = 480.0;
const PADDING: f64 = 40.0;
const SAMPLES: usize = 500;

/// Draw the plot of `expr` (which should use `x` as free variable) onto the
/// canvas element with the given id.  `vars` supplies any stored variables
/// (excluding `x` which is swept).  Returns an error message on failure.
pub(crate) fn draw_plot(
    canvas_id: &str,
    expr: &str,
    vars: &HashMap<String, f64>,
    x_min: f64,
    x_max: f64,
) -> Result<(), String> {
    let document = web_sys::window()
        .ok_or("no window")?
        .document()
        .ok_or("no document")?;

    let canvas = document
        .get_element_by_id(canvas_id)
        .ok_or("canvas not found")?
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| "element is not a canvas")?;

    canvas.set_width(CANVAS_W as u32);
    canvas.set_height(CANVAS_H as u32);

    let ctx = canvas
        .get_context("2d")
        .map_err(|_| "getContext failed")?
        .ok_or("no 2d context")?
        .dyn_into::<CanvasRenderingContext2d>()
        .map_err(|_| "not a CanvasRenderingContext2d")?;

    // Clear
    ctx.set_fill_style_str("#000000");
    ctx.fill_rect(0.0, 0.0, CANVAS_W, CANVAS_H);

    if x_min >= x_max {
        draw_message(&ctx, "x min must be less than x max");
        return Ok(());
    }

    // Sample points
    let step = (x_max - x_min) / SAMPLES as f64;
    let mut points: Vec<Option<(f64, f64)>> = Vec::with_capacity(SAMPLES + 1);
    let mut y_vals: Vec<f64> = Vec::new();

    for i in 0..=SAMPLES {
        let x = x_min + i as f64 * step;
        let mut eval_vars = vars.clone();
        eval_vars.insert("x".into(), x);
        match evaluate(expr, &eval_vars) {
            Ok(y) if y.is_finite() => {
                points.push(Some((x, y)));
                y_vals.push(y);
            }
            _ => points.push(None),
        }
    }

    if y_vals.is_empty() {
        draw_message(&ctx, "No valid points to plot");
        return Ok(());
    }

    // Compute y range with padding
    let y_min_data = y_vals.iter().copied().fold(f64::INFINITY, f64::min);
    let y_max_data = y_vals.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let (y_min, y_max) = if (y_max_data - y_min_data).abs() < 1e-12 {
        (y_min_data - 1.0, y_max_data + 1.0)
    } else {
        let margin = (y_max_data - y_min_data) * 0.08;
        (y_min_data - margin, y_max_data + margin)
    };

    let plot_w = CANVAS_W - 2.0 * PADDING;
    let plot_h = CANVAS_H - 2.0 * PADDING;

    let to_px_x = |x: f64| PADDING + (x - x_min) / (x_max - x_min) * plot_w;
    let to_px_y = |y: f64| PADDING + (1.0 - (y - y_min) / (y_max - y_min)) * plot_h;

    // Draw grid and axes
    draw_axes(&ctx, x_min, x_max, y_min, y_max, &to_px_x, &to_px_y);

    // Draw curve
    ctx.set_stroke_style_str("#00ff41");
    ctx.set_shadow_color("#00ff41");
    ctx.set_shadow_blur(6.0);
    ctx.set_line_width(2.0);
    ctx.begin_path();

    let mut pen_down = false;
    for pt in &points {
        match pt {
            Some((x, y)) => {
                let px = to_px_x(*x);
                let py = to_px_y(*y);
                // Clip to plot area
                if px >= PADDING
                    && px <= CANVAS_W - PADDING
                    && py >= PADDING
                    && py <= CANVAS_H - PADDING
                {
                    if pen_down {
                        ctx.line_to(px, py);
                    } else {
                        ctx.move_to(px, py);
                        pen_down = true;
                    }
                } else {
                    pen_down = false;
                }
            }
            None => {
                pen_down = false;
            }
        }
    }
    ctx.stroke();
    ctx.set_shadow_blur(0.0);

    Ok(())
}

fn draw_axes(
    ctx: &CanvasRenderingContext2d,
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
    to_px_x: &dyn Fn(f64) -> f64,
    to_px_y: &dyn Fn(f64) -> f64,
) {
    let plot_left = PADDING;
    let plot_right = CANVAS_W - PADDING;
    let plot_top = PADDING;
    let plot_bottom = CANVAS_H - PADDING;

    // Grid lines
    ctx.set_stroke_style_str("rgba(0, 255, 65, 0.08)");
    ctx.set_line_width(1.0);

    let x_ticks = nice_ticks(x_min, x_max, 8);
    let y_ticks = nice_ticks(y_min, y_max, 6);

    for &xt in &x_ticks {
        let px = to_px_x(xt);
        if px > plot_left && px < plot_right {
            ctx.begin_path();
            ctx.move_to(px, plot_top);
            ctx.line_to(px, plot_bottom);
            ctx.stroke();
        }
    }
    for &yt in &y_ticks {
        let py = to_px_y(yt);
        if py > plot_top && py < plot_bottom {
            ctx.begin_path();
            ctx.move_to(plot_left, py);
            ctx.line_to(plot_right, py);
            ctx.stroke();
        }
    }

    // Draw x=0 and y=0 axes if visible
    ctx.set_stroke_style_str("rgba(0, 255, 65, 0.3)");
    ctx.set_line_width(1.0);
    if x_min <= 0.0 && x_max >= 0.0 {
        let px = to_px_x(0.0);
        ctx.begin_path();
        ctx.move_to(px, plot_top);
        ctx.line_to(px, plot_bottom);
        ctx.stroke();
    }
    if y_min <= 0.0 && y_max >= 0.0 {
        let py = to_px_y(0.0);
        ctx.begin_path();
        ctx.move_to(plot_left, py);
        ctx.line_to(plot_right, py);
        ctx.stroke();
    }

    // Border
    ctx.set_stroke_style_str("rgba(0, 255, 65, 0.25)");
    ctx.stroke_rect(
        plot_left,
        plot_top,
        plot_right - plot_left,
        plot_bottom - plot_top,
    );

    // Tick labels
    ctx.set_fill_style_str("rgba(0, 255, 65, 0.6)");
    ctx.set_font("10px 'Share Tech Mono', monospace");
    ctx.set_text_align("center");
    ctx.set_text_baseline("top");
    for &xt in &x_ticks {
        let px = to_px_x(xt);
        if px >= plot_left && px <= plot_right {
            let _ = ctx.fill_text(&format_tick(xt), px, plot_bottom + 4.0);
        }
    }

    ctx.set_text_align("right");
    ctx.set_text_baseline("middle");
    for &yt in &y_ticks {
        let py = to_px_y(yt);
        if py >= plot_top && py <= plot_bottom {
            let _ = ctx.fill_text(&format_tick(yt), plot_left - 4.0, py);
        }
    }
}

fn draw_message(ctx: &CanvasRenderingContext2d, msg: &str) {
    ctx.set_fill_style_str("rgba(255, 32, 32, 0.7)");
    ctx.set_font("14px 'Share Tech Mono', monospace");
    ctx.set_text_align("center");
    ctx.set_text_baseline("middle");
    let _ = ctx.fill_text(msg, CANVAS_W / 2.0, CANVAS_H / 2.0);
}

/// Produce ~`target_count` "nice" tick values spanning `min..max`.
fn nice_ticks(min: f64, max: f64, target_count: usize) -> Vec<f64> {
    let range = max - min;
    if range <= 0.0 || !range.is_finite() {
        return vec![];
    }
    let rough = range / target_count as f64;
    let mag = 10f64.powf(rough.log10().floor());
    let frac = rough / mag;
    let nice = if frac <= 1.5 {
        mag
    } else if frac <= 3.5 {
        2.0 * mag
    } else if frac <= 7.5 {
        5.0 * mag
    } else {
        10.0 * mag
    };

    let start = (min / nice).ceil() as i64;
    let end = (max / nice).floor() as i64;
    (start..=end).map(|i| i as f64 * nice).collect()
}

fn format_tick(v: f64) -> String {
    if v == 0.0 {
        "0".into()
    } else if v.abs() >= 1e10 || v.abs() < 0.001 {
        format!("{:.2e}", v)
    } else {
        let s = format!("{:.4}", v);
        let s = s.trim_end_matches('0');
        let s = s.trim_end_matches('.');
        s.to_string()
    }
}
