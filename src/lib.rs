mod eval;
mod plot;

use eval::{ParseError, error_pointer, evaluate, fmt_value, validate_var_name};
use leptos::*;
use plot::draw_plot;
use std::collections::HashMap;

// ── Leptos component ──────────────────────────────────────────────────────────

#[component]
pub fn App(cx: Scope) -> impl IntoView {
    let (input, set_input) = create_signal(cx, String::new());
    let (vars, set_vars) = create_signal(cx, Vec::<(String, f64)>::new());
    let (storing, set_storing) = create_signal(cx, false);
    let (var_name, set_var_name) = create_signal(cx, String::new());
    let (var_error, set_var_error) = create_signal(cx, String::new());

    // ── Plot state ──
    let (plot_mode, set_plot_mode) = create_signal(cx, false);
    let (x_min_str, set_x_min_str) = create_signal(cx, "-10".to_string());
    let (x_max_str, set_x_max_str) = create_signal(cx, "10".to_string());
    let (plot_error, set_plot_error) = create_signal(cx, String::new());

    // memoised HashMap built from the vars Vec
    let vars_map = create_memo(cx, move |_| -> HashMap<String, f64> {
        vars.get().into_iter().collect()
    });

    // memoised evaluation result — computed once per input/vars change
    let result_value = create_memo(cx, move |_| -> Option<Result<f64, ParseError>> {
        let raw = input.get();
        if raw.trim().is_empty() {
            return None;
        }
        Some(evaluate(&raw, &vars_map.get()))
    });

    let result_str = move || match result_value.get() {
        None => String::new(),
        Some(Ok(v)) => fmt_value(v),
        Some(Err(ref e)) => e.msg.clone(),
    };

    let is_valid_result = move || matches!(result_value.get(), Some(Ok(_)));

    // error pointer string (empty when no positional error)
    let error_pointer_str = move || -> String {
        match result_value.get() {
            Some(Err(ref e)) => {
                if let Some(pos) = e.pos {
                    error_pointer(&input.get(), pos)
                } else {
                    String::new()
                }
            }
            _ => String::new(),
        }
    };

    let has_error = move || matches!(result_value.get(), Some(Err(_)));

    // shared helper: validate name + store variable
    let try_store_variable = move |name: String| -> Result<(), String> {
        validate_var_name(&name)?;
        if let Some(Ok(v)) = result_value.get() {
            let mut current = vars.get();
            if let Some(entry) = current.iter_mut().find(|(n, _)| n == &name) {
                entry.1 = v;
            } else {
                current.push((name, v));
            }
            set_vars.set(current);
            set_storing.set(false);
            set_var_error.set(String::new());
            Ok(())
        } else {
            Err("No valid result to store".into())
        }
    };

    let on_store_click = move |_| {
        if !storing.get() {
            set_var_name.set(String::new());
            set_var_error.set(String::new());
            set_storing.set(true);
        }
    };

    let on_confirm_store = move |_| {
        if let Err(msg) = try_store_variable(var_name.get()) {
            set_var_error.set(msg);
        }
    };

    let on_cancel_store = move |_| {
        set_storing.set(false);
        set_var_error.set(String::new());
    };

    let on_delete_var = move |name: String| {
        set_vars.update(|v| v.retain(|(n, _)| n != &name));
    };

    // ── Redraw plot whenever expression, vars, range, or mode changes ──
    create_effect(cx, move |_| {
        if !plot_mode.get() {
            return;
        }
        let expr = input.get();
        if expr.trim().is_empty() {
            return;
        }
        let xmin = x_min_str.get().parse::<f64>().unwrap_or(-10.0);
        let xmax = x_max_str.get().parse::<f64>().unwrap_or(10.0);
        let vm = vars_map.get();
        match draw_plot("plot-canvas", &expr, &vm, xmin, xmax) {
            Ok(()) => set_plot_error.set(String::new()),
            Err(msg) => set_plot_error.set(msg),
        }
    });

    view! { cx,
        <div class="calc-card">
            <p class="calc-title">"Neo Calculator"</p>

            // ── Tab bar ──
            <div class="tab-bar">
                <button
                    class="tab-btn"
                    class:tab-active=move || !plot_mode.get()
                    on:click=move |_| set_plot_mode.set(false)
                >"Calculate"</button>
                <button
                    class="tab-btn"
                    class:tab-active=move || plot_mode.get()
                    on:click=move |_| set_plot_mode.set(true)
                >"Plot"</button>
            </div>

            <div class="input-wrapper">
                <input
                    class="calc-expr-input"
                    class:has-error=move || has_error() && !plot_mode.get()
                    type="text"
                    placeholder=move || {
                        if plot_mode.get() {
                            "e.g. sin(x), x^2 - 3x + 1"
                        } else {
                            "e.g. 2pi + sin(3!) or a*2"
                        }
                    }
                    prop:value={input.get()}
                    on:input=move |ev| {
                        set_input.set(event_target_value(&ev));
                        set_storing.set(false);
                    }
                />
                {move || {
                    if plot_mode.get() { return None; }
                    let ptr = error_pointer_str();
                    (!ptr.is_empty()).then(|| view! { cx,
                        <div class="error-pointer">{ptr}</div>
                    })
                }}
            </div>

            // ── Calculate mode ──
            {move || (!plot_mode.get()).then(|| view! { cx,
                <div class="calc-result">
                    <span class="calc-result-label">"Result"</span>
                    <span
                        class="calc-result-value"
                        class:calc-result-error=has_error
                    >{result_str}</span>
                </div>

                // ── Store controls ──
                <div class="store-row">
                    <button
                        class="btn-store"
                        disabled=move || !is_valid_result()
                        on:click=on_store_click
                    >
                        "Store"
                    </button>
                    {move || storing.get().then(|| view! { cx,
                        <div class="store-input-row">
                            <input
                                class="store-name-input"
                                type="text"
                                placeholder="variable name"
                                on:input=move |ev| set_var_name.set(event_target_value(&ev))
                                on:keydown=move |ev| {
                                    if ev.key() == "Enter" {
                                        if let Err(msg) = try_store_variable(var_name.get()) {
                                            set_var_error.set(msg);
                                        }
                                    }
                                    if ev.key() == "Escape" {
                                        set_storing.set(false);
                                        set_var_error.set(String::new());
                                    }
                                }
                            />
                            <button class="btn-confirm" on:click=on_confirm_store>"✓"</button>
                            <button class="btn-cancel"  on:click=on_cancel_store>"✕"</button>
                        </div>
                    })}
                </div>
                {move || (!var_error.get().is_empty()).then(|| view! { cx,
                    <p class="store-error">{var_error.get()}</p>
                })}
            })}

            // ── Plot mode ──
            {move || plot_mode.get().then(|| view! { cx,
                <div class="plot-range-row">
                    <label class="plot-range-label">"x min"</label>
                    <input
                        class="plot-range-input"
                        type="text"
                        prop:value=move || x_min_str.get()
                        on:input=move |ev| set_x_min_str.set(event_target_value(&ev))
                    />
                    <label class="plot-range-label">"x max"</label>
                    <input
                        class="plot-range-input"
                        type="text"
                        prop:value=move || x_max_str.get()
                        on:input=move |ev| set_x_max_str.set(event_target_value(&ev))
                    />
                </div>
                <div class="plot-canvas-wrapper">
                    <canvas id="plot-canvas"></canvas>
                </div>
                {move || (!plot_error.get().is_empty()).then(|| view! { cx,
                    <p class="store-error">{plot_error.get()}</p>
                })}
            })}

            // ── Variables table ──
            {move || (!vars.get().is_empty()).then(|| view! { cx,
                <div class="vars-header">
                    <span class="vars-title">"Variables"</span>
                    <button class="btn-clear-all" on:click=move |_| set_vars.set(Vec::new())>"Clear all"</button>
                </div>
                <table class="vars-table">
                    <thead>
                        <tr>
                            <th>"Variable"</th>
                            <th>"Value"</th>
                            <th></th>
                        </tr>
                    </thead>
                    <tbody>
                        {vars.get().into_iter().map(|(name, val)| {
                            let name_clone = name.clone();
                            view! { cx,
                                <tr>
                                    <td class="var-name">{name.clone()}</td>
                                    <td class="var-val">{fmt_value(val)}</td>
                                    <td>
                                        <button
                                            class="btn-delete"
                                            on:click=move |_| on_delete_var(name_clone.clone())
                                        >"✕"</button>
                                    </td>
                                </tr>
                            }
                        }).collect::<Vec<_>>()}
                    </tbody>
                </table>
            })}
        </div>
    }
}

// exported entry point for the generated wasm module
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(|cx| view! { cx, <App/> });
}
