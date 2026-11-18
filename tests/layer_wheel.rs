// 回归：设置浮层（Foreground 层）盖在终端（Background 层）上时，浮层里拨滚轮
// **不许**连带滚终端。判据必须按层算（rect_contains_pointer），只比坐标会误中。
// cargo test --test layer_wheel
use eframe::egui;
use egui::{Context, Event, Id, MouseWheelUnit, Order, Pos2, Rect, TouchPhase, Vec2};

/// 复刻 app.rs 的绘制顺序：中央面板终端 → Foreground 设置浮层。
/// 返回终端是否（按 terminal.rs 的判据）消费了这帧滚轮。
fn frame(ctx: &Context, events: Vec<Event>, open: bool) -> bool {
    let mut term_consumed = false;
    let raw = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(800.0, 600.0))),
        events,
        ..Default::default()
    };
    let mut out = ctx.run_ui(raw, |ui| {
        egui::CentralPanel::default().show(ui, |ui| {
            let (r, _) = ui.allocate_exact_size(egui::vec2(800.0, 600.0), egui::Sense::click());
            term_consumed = ui.rect_contains_pointer(r);
            if term_consumed {
                ui.input_mut(|i| i.smooth_scroll_delta.y = 0.0);
            }
        });
        if open {
            egui::Area::new(Id::new("settings_win"))
                .order(Order::Foreground)
                .current_pos(egui::pos2(200.0, 100.0))
                .interactable(true)
                .show(ui.ctx(), |ui| {
                    ui.set_width(200.0);
                    ui.set_height(200.0);
                    egui::ScrollArea::vertical()
                        .id_salt("settings_scroll")
                        .show(ui, |ui| {
                            for i in 0..40 {
                                ui.label(format!("设置项 {i}"));
                            }
                        });
                });
        }
    });
    out.textures_delta.clear();
    term_consumed
}

fn wheel(pos: Pos2) -> Vec<Event> {
    vec![
        Event::PointerMoved(pos),
        Event::MouseWheel {
            unit: MouseWheelUnit::Line,
            delta: Vec2::new(0.0, -1.0),
            phase: TouchPhase::Move,
            modifiers: egui::Modifiers::default(),
        },
    ]
}

/// 头几帧做布局（clip rect / 层登记），命中判定才稳定。
fn settle(ctx: &Context, pos: Pos2, open: bool) {
    for _ in 0..3 {
        frame(ctx, vec![Event::PointerMoved(pos)], open);
    }
}

#[test]
fn wheel_over_overlay_does_not_hit_terminal() {
    let ctx = Context::default();
    ctx.set_fonts(egui::FontDefinitions::empty());
    // 250,150 同时落在终端矩形和浮层矩形里——但浮层在上，终端不该命中。
    let pos = egui::pos2(250.0, 150.0);
    settle(&ctx, pos, true);
    assert!(
        !frame(&ctx, wheel(pos), true),
        "浮层之上不该被终端按层命中"
    );
}

#[test]
fn wheel_over_plain_terminal_hits_terminal() {
    let ctx = Context::default();
    ctx.set_fonts(egui::FontDefinitions::empty());
    // 700,500 在终端矩形里、浮层外。
    let pos = egui::pos2(700.0, 500.0);
    settle(&ctx, pos, true);
    assert!(
        frame(&ctx, wheel(pos), true),
        "指针在终端上时该由终端按层命中"
    );
}
