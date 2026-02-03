use std::fmt::Write;

use gam::{Gam, GlyphStyle, Gid};
use gam::menu::*;

use crate::pomodoro::PomodoroState;
use crate::stopwatch::StopwatchState;
use crate::countdown::CountdownState;
use crate::alerts::AlertConfig;
use timer_core::{format_ms, format_hms_cs};

pub fn clear_screen(gam: &Gam, content: Gid, screensize: Point) {
    gam.draw_rectangle(
        content,
        Rectangle::new_with_style(
            Point::new(0, 0),
            screensize,
            DrawStyle {
                fill_color: Some(PixelColor::Light),
                stroke_color: None,
                stroke_width: 0,
            },
        ),
    )
    .expect("can't clear");
}

pub fn draw_menu(
    gam: &Gam,
    content: Gid,
    screensize: Point,
    items: &[&str],
    cursor: usize,
) {
    clear_screen(gam, content, screensize);

    let mut title_tv = TextView::new(
        content,
        TextBounds::BoundingBox(Rectangle::new_coords(12, 12, screensize.x - 12, 40)),
    );
    title_tv.style = GlyphStyle::Bold;
    title_tv.clear_area = true;
    write!(title_tv.text, "MENU").unwrap();
    gam.post_textview(&mut title_tv).expect("can't post title");

    let line_height = 30;
    let list_top = 52;

    for (i, item) in items.iter().enumerate() {
        let y = list_top + (i as isize) * line_height;
        let marker = if i == cursor { "> " } else { "  " };

        let mut tv = TextView::new(
            content,
            TextBounds::BoundingBox(Rectangle::new_coords(16, y, screensize.x - 16, y + line_height - 2)),
        );
        tv.style = GlyphStyle::Regular;
        tv.clear_area = true;
        write!(tv.text, "{}{}", marker, item).unwrap();
        gam.post_textview(&mut tv).expect("can't post menu item");
    }

    let mut nav_tv = TextView::new(
        content,
        TextBounds::BoundingBox(Rectangle::new_coords(12, screensize.y - 40, screensize.x - 12, screensize.y - 10)),
    );
    nav_tv.style = GlyphStyle::Small;
    nav_tv.clear_area = true;
    write!(nav_tv.text, "arrows=select  ENTER=open  F4=close").unwrap();
    gam.post_textview(&mut nav_tv).expect("can't post footer");

    gam.redraw().expect("can't redraw");
}

pub fn draw_help(gam: &Gam, content: Gid, screensize: Point, help_text: &str) {
    clear_screen(gam, content, screensize);

    let line_height = 20;
    let mut y = 16isize;

    for line in help_text.lines() {
        if y + line_height > screensize.y - 40 {
            break;
        }
        let style = if y == 16 { GlyphStyle::Bold } else { GlyphStyle::Small };
        let mut tv = TextView::new(
            content,
            TextBounds::BoundingBox(Rectangle::new_coords(16, y, screensize.x - 16, y + line_height - 2)),
        );
        tv.style = style;
        tv.clear_area = true;
        write!(tv.text, "{}", line).unwrap();
        gam.post_textview(&mut tv).expect("can't post help line");
        y += line_height;
    }

    let mut nav_tv = TextView::new(
        content,
        TextBounds::BoundingBox(Rectangle::new_coords(12, screensize.y - 30, screensize.x - 12, screensize.y - 8)),
    );
    nav_tv.style = GlyphStyle::Small;
    nav_tv.clear_area = true;
    write!(nav_tv.text, "Press any key to close").unwrap();
    gam.post_textview(&mut nav_tv).expect("can't post footer");

    gam.redraw().expect("can't redraw");
}

pub fn draw_confirm_exit(gam: &Gam, content: Gid, screensize: Point) {
    clear_screen(gam, content, screensize);

    let mut title_tv = TextView::new(
        content,
        TextBounds::BoundingBox(Rectangle::new_coords(12, 40, screensize.x - 12, 70)),
    );
    title_tv.style = GlyphStyle::Bold;
    title_tv.clear_area = true;
    write!(title_tv.text, "Timer Running").unwrap();
    gam.post_textview(&mut title_tv).expect("can't post title");

    let mut msg_tv = TextView::new(
        content,
        TextBounds::BoundingBox(Rectangle::new_coords(12, 90, screensize.x - 12, 150)),
    );
    msg_tv.style = GlyphStyle::Regular;
    msg_tv.clear_area = true;
    write!(msg_tv.text, "A timer is still running.\nExit anyway?").unwrap();
    gam.post_textview(&mut msg_tv).expect("can't post message");

    let mut nav_tv = TextView::new(
        content,
        TextBounds::BoundingBox(Rectangle::new_coords(12, 170, screensize.x - 12, 210)),
    );
    nav_tv.style = GlyphStyle::Regular;
    nav_tv.clear_area = true;
    write!(nav_tv.text, "  y = Stop & exit\n  n = Cancel\n  F4 = Cancel").unwrap();
    gam.post_textview(&mut nav_tv).expect("can't post options");

    gam.redraw().expect("can't redraw");
}

pub fn draw_mode_select(gam: &Gam, content: Gid, screensize: Point, cursor: usize) {
    clear_screen(gam, content, screensize);

    let mut title_tv = TextView::new(
        content,
        TextBounds::BoundingBox(Rectangle::new_coords(12, 8, screensize.x - 12, 36)),
    );
    title_tv.style = GlyphStyle::Bold;
    title_tv.clear_area = true;
    write!(title_tv.text, "TIMERS").unwrap();
    gam.post_textview(&mut title_tv).expect("can't post title");

    let modes = ["Pomodoro", "Stopwatch", "Countdown"];
    let line_height = 32;
    let list_top = 60;

    for (i, mode) in modes.iter().enumerate() {
        let y = list_top + (i as isize) * line_height;
        let marker = if i == cursor { "> " } else { "  " };

        let mut tv = TextView::new(
            content,
            TextBounds::BoundingBox(Rectangle::new_coords(20, y, screensize.x - 20, y + line_height - 2)),
        );
        tv.style = GlyphStyle::Regular;
        tv.clear_area = true;
        write!(tv.text, "{}{}", marker, mode).unwrap();
        gam.post_textview(&mut tv).expect("can't post mode item");
    }

    let mut nav_tv = TextView::new(
        content,
        TextBounds::BoundingBox(Rectangle::new_coords(12, screensize.y - 50, screensize.x - 12, screensize.y - 10)),
    );
    nav_tv.style = GlyphStyle::Small;
    nav_tv.clear_area = true;
    write!(nav_tv.text, "F1=menu F4=quit  ENTER=open  s=settings").unwrap();
    gam.post_textview(&mut nav_tv).expect("can't post footer");

    gam.redraw().expect("can't redraw");
}

pub fn draw_pomodoro(gam: &Gam, content: Gid, screensize: Point, state: &PomodoroState, now_ms: u64) {
    clear_screen(gam, content, screensize);

    // Header
    let mut title_tv = TextView::new(
        content,
        TextBounds::BoundingBox(Rectangle::new_coords(12, 8, screensize.x - 12, 36)),
    );
    title_tv.style = GlyphStyle::Bold;
    title_tv.clear_area = true;
    write!(
        title_tv.text, "POMODORO  [{} {}/{}]",
        state.phase_label(),
        state.current_cycle + 1,
        state.cycles_before_long
    ).unwrap();
    gam.post_textview(&mut title_tv).expect("can't post title");

    // Time display
    let remaining = state.timer.remaining_ms(now_ms).unwrap_or(0);
    let time_str = format_ms(remaining);
    let mut time_tv = TextView::new(
        content,
        TextBounds::BoundingBox(Rectangle::new_coords(40, 70, screensize.x - 40, 120)),
    );
    time_tv.style = GlyphStyle::Bold;
    time_tv.clear_area = true;
    write!(time_tv.text, "     {}", time_str).unwrap();
    gam.post_textview(&mut time_tv).expect("can't post time");

    // Progress bar
    let bar_left = 30;
    let bar_right = screensize.x - 30;
    let bar_top = 135;
    let bar_bottom = bar_top + 16;
    let bar_width = bar_right - bar_left;

    // Bar outline
    gam.draw_rectangle(
        content,
        Rectangle::new_with_style(
            Point::new(bar_left, bar_top),
            Point::new(bar_right, bar_bottom),
            DrawStyle {
                fill_color: None,
                stroke_color: Some(PixelColor::Dark),
                stroke_width: 1,
            },
        ),
    ).expect("can't draw bar outline");

    // Bar fill
    let progress = state.progress_fraction(now_ms);
    let fill_width = (bar_width as f32 * progress) as isize;
    if fill_width > 0 {
        gam.draw_rectangle(
            content,
            Rectangle::new_with_style(
                Point::new(bar_left + 1, bar_top + 1),
                Point::new(bar_left + 1 + fill_width, bar_bottom - 1),
                DrawStyle {
                    fill_color: Some(PixelColor::Dark),
                    stroke_color: None,
                    stroke_width: 0,
                },
            ),
        ).expect("can't draw bar fill");
    }

    // Session counter
    let mut session_tv = TextView::new(
        content,
        TextBounds::BoundingBox(Rectangle::new_coords(12, 170, screensize.x - 12, 195)),
    );
    session_tv.style = GlyphStyle::Small;
    session_tv.clear_area = true;
    write!(session_tv.text, "Sessions completed: {}", state.total_completed).unwrap();
    gam.post_textview(&mut session_tv).expect("can't post session");

    // Footer
    let mut nav_tv = TextView::new(
        content,
        TextBounds::BoundingBox(Rectangle::new_coords(12, screensize.y - 50, screensize.x - 12, screensize.y - 10)),
    );
    nav_tv.style = GlyphStyle::Small;
    nav_tv.clear_area = true;
    write!(nav_tv.text, "F2=start/pause  F3=reset  F4=back\nF1=menu  s=settings").unwrap();
    gam.post_textview(&mut nav_tv).expect("can't post footer");

    gam.redraw().expect("can't redraw");
}

pub fn draw_stopwatch(gam: &Gam, content: Gid, screensize: Point, state: &StopwatchState, now_ms: u64) {
    clear_screen(gam, content, screensize);

    // Header
    let mut title_tv = TextView::new(
        content,
        TextBounds::BoundingBox(Rectangle::new_coords(12, 8, screensize.x - 12, 36)),
    );
    title_tv.style = GlyphStyle::Bold;
    title_tv.clear_area = true;
    write!(title_tv.text, "STOPWATCH").unwrap();
    gam.post_textview(&mut title_tv).expect("can't post title");

    // Time display
    let elapsed = state.timer.elapsed_ms(now_ms);
    let time_str = format_hms_cs(elapsed);
    let mut time_tv = TextView::new(
        content,
        TextBounds::BoundingBox(Rectangle::new_coords(20, 50, screensize.x - 20, 90)),
    );
    time_tv.style = GlyphStyle::Bold;
    time_tv.clear_area = true;
    write!(time_tv.text, "  {}", time_str).unwrap();
    gam.post_textview(&mut time_tv).expect("can't post time");

    // Lap list (most recent first)
    let line_height = 22;
    let list_top = 100;
    let list_bottom = screensize.y - 60;
    let max_visible = ((list_bottom - list_top) / line_height) as usize;

    if !state.laps.is_empty() {
        let visible_count = max_visible.min(state.laps.len());
        let start = if state.laps.len() > state.lap_scroll_offset {
            state.laps.len() - state.lap_scroll_offset
        } else {
            0
        };

        for i in 0..visible_count {
            let lap_idx = if start > i { start - 1 - i } else { break };
            if lap_idx >= state.laps.len() {
                break;
            }
            let y = list_top + (i as isize) * line_height;
            let lap_time = format_hms_cs(state.laps[lap_idx]);

            let mut tv = TextView::new(
                content,
                TextBounds::BoundingBox(Rectangle::new_coords(20, y, screensize.x - 20, y + line_height - 2)),
            );
            tv.style = GlyphStyle::Small;
            tv.clear_area = true;
            write!(tv.text, "Lap {:2}: {}", lap_idx + 1, lap_time).unwrap();
            gam.post_textview(&mut tv).expect("can't post lap");
        }
    }

    // Footer
    let mut nav_tv = TextView::new(
        content,
        TextBounds::BoundingBox(Rectangle::new_coords(12, screensize.y - 50, screensize.x - 12, screensize.y - 10)),
    );
    nav_tv.style = GlyphStyle::Small;
    nav_tv.clear_area = true;
    write!(nav_tv.text, "F2=start/pause  F3=reset  F4=back\nF1=menu  l=lap").unwrap();
    gam.post_textview(&mut nav_tv).expect("can't post footer");

    gam.redraw().expect("can't redraw");
}

pub fn draw_countdown_list(gam: &Gam, content: Gid, screensize: Point, state: &CountdownState) {
    clear_screen(gam, content, screensize);

    // Header
    let mut title_tv = TextView::new(
        content,
        TextBounds::BoundingBox(Rectangle::new_coords(12, 8, screensize.x - 12, 36)),
    );
    title_tv.style = GlyphStyle::Bold;
    title_tv.clear_area = true;
    write!(title_tv.text, "COUNTDOWNS").unwrap();
    gam.post_textview(&mut title_tv).expect("can't post title");

    // List
    let line_height = 28;
    let list_top = 44;
    let list_bottom = screensize.y - 60;
    let max_visible = ((list_bottom - list_top) / line_height) as usize;

    if state.entries.is_empty() {
        let mut tv = TextView::new(
            content,
            TextBounds::BoundingBox(Rectangle::new_coords(20, list_top + 10, screensize.x - 20, list_top + 40)),
        );
        tv.style = GlyphStyle::Regular;
        tv.clear_area = true;
        write!(tv.text, "No timers. Press 'n' to add.").unwrap();
        gam.post_textview(&mut tv).expect("can't post empty");
    } else {
        let visible_end = max_visible.min(state.entries.len());
        for (i, entry) in state.entries[..visible_end].iter().enumerate() {
            let y = list_top + (i as isize) * line_height;
            let marker = if i == state.cursor { "> " } else { "  " };
            let duration_str = format_ms(entry.duration_ms);

            let mut tv = TextView::new(
                content,
                TextBounds::BoundingBox(Rectangle::new_coords(12, y, screensize.x - 12, y + line_height - 2)),
            );
            tv.style = GlyphStyle::Regular;
            tv.clear_area = true;
            write!(tv.text, "{}{:<14} {}", marker, entry.name, duration_str).unwrap();
            gam.post_textview(&mut tv).expect("can't post entry");
        }
    }

    // Footer
    let mut nav_tv = TextView::new(
        content,
        TextBounds::BoundingBox(Rectangle::new_coords(12, screensize.y - 50, screensize.x - 12, screensize.y - 10)),
    );
    nav_tv.style = GlyphStyle::Small;
    nav_tv.clear_area = true;
    write!(nav_tv.text, "F1=menu F4=back  ENTER=start\nn=new  d=delete").unwrap();
    gam.post_textview(&mut nav_tv).expect("can't post footer");

    gam.redraw().expect("can't redraw");
}

pub fn draw_countdown_running(gam: &Gam, content: Gid, screensize: Point, state: &CountdownState, now_ms: u64) {
    clear_screen(gam, content, screensize);

    let name = state.active_name().unwrap_or("Timer");

    // Header
    let mut title_tv = TextView::new(
        content,
        TextBounds::BoundingBox(Rectangle::new_coords(12, 8, screensize.x - 12, 36)),
    );
    title_tv.style = GlyphStyle::Bold;
    title_tv.clear_area = true;
    write!(title_tv.text, "COUNTDOWN: {}", name).unwrap();
    gam.post_textview(&mut title_tv).expect("can't post title");

    // Time display
    let remaining = state.active_timer.as_ref()
        .and_then(|t| t.remaining_ms(now_ms))
        .unwrap_or(0);
    let time_str = format_ms(remaining);
    let mut time_tv = TextView::new(
        content,
        TextBounds::BoundingBox(Rectangle::new_coords(40, 70, screensize.x - 40, 120)),
    );
    time_tv.style = GlyphStyle::Bold;
    time_tv.clear_area = true;
    write!(time_tv.text, "     {}", time_str).unwrap();
    gam.post_textview(&mut time_tv).expect("can't post time");

    // Progress bar
    let bar_left = 30;
    let bar_right = screensize.x - 30;
    let bar_top = 135;
    let bar_bottom = bar_top + 16;
    let bar_width = bar_right - bar_left;

    gam.draw_rectangle(
        content,
        Rectangle::new_with_style(
            Point::new(bar_left, bar_top),
            Point::new(bar_right, bar_bottom),
            DrawStyle {
                fill_color: None,
                stroke_color: Some(PixelColor::Dark),
                stroke_width: 1,
            },
        ),
    ).expect("can't draw bar outline");

    let progress = state.progress_fraction(now_ms);
    let fill_width = (bar_width as f32 * progress) as isize;
    if fill_width > 0 {
        gam.draw_rectangle(
            content,
            Rectangle::new_with_style(
                Point::new(bar_left + 1, bar_top + 1),
                Point::new(bar_left + 1 + fill_width, bar_bottom - 1),
                DrawStyle {
                    fill_color: Some(PixelColor::Dark),
                    stroke_color: None,
                    stroke_width: 0,
                },
            ),
        ).expect("can't draw bar fill");
    }

    // Footer
    let mut nav_tv = TextView::new(
        content,
        TextBounds::BoundingBox(Rectangle::new_coords(12, screensize.y - 50, screensize.x - 12, screensize.y - 10)),
    );
    nav_tv.style = GlyphStyle::Small;
    nav_tv.clear_area = true;
    write!(nav_tv.text, "F2=pause/resume  F3=reset\nF4=back  F1=menu").unwrap();
    gam.post_textview(&mut nav_tv).expect("can't post footer");

    gam.redraw().expect("can't redraw");
}

pub fn draw_settings(gam: &Gam, content: Gid, screensize: Point, config: &AlertConfig, cursor: usize) {
    clear_screen(gam, content, screensize);

    let mut title_tv = TextView::new(
        content,
        TextBounds::BoundingBox(Rectangle::new_coords(12, 8, screensize.x - 12, 36)),
    );
    title_tv.style = GlyphStyle::Bold;
    title_tv.clear_area = true;
    write!(title_tv.text, "SETTINGS").unwrap();
    gam.post_textview(&mut title_tv).expect("can't post title");

    let line_height = 30;
    let list_top = 60;

    // Alert settings
    let alert_items = [
        ("Vibration", config.vibration),
        ("Notification", config.notification),
        ("Audio", config.audio),
    ];

    for (i, (label, enabled)) in alert_items.iter().enumerate() {
        let y = list_top + (i as isize) * line_height;
        let marker = if i == cursor { "> " } else { "  " };
        let status = if *enabled { "[ON]" } else { "[OFF]" };

        let mut tv = TextView::new(
            content,
            TextBounds::BoundingBox(Rectangle::new_coords(12, y, screensize.x - 12, y + line_height - 2)),
        );
        tv.style = GlyphStyle::Regular;
        tv.clear_area = true;
        write!(tv.text, "{}{:<16} {}", marker, label, status).unwrap();
        gam.post_textview(&mut tv).expect("can't post setting");
    }

    // Configure Pomodoro option
    let pom_y = list_top + 3 * line_height;
    let pom_marker = if cursor == 3 { "> " } else { "  " };
    let mut pom_tv = TextView::new(
        content,
        TextBounds::BoundingBox(Rectangle::new_coords(12, pom_y, screensize.x - 12, pom_y + line_height - 2)),
    );
    pom_tv.style = GlyphStyle::Regular;
    pom_tv.clear_area = true;
    write!(pom_tv.text, "{}Configure Pomodoro...", pom_marker).unwrap();
    gam.post_textview(&mut pom_tv).expect("can't post pom setting");

    let mut nav_tv = TextView::new(
        content,
        TextBounds::BoundingBox(Rectangle::new_coords(12, screensize.y - 50, screensize.x - 12, screensize.y - 10)),
    );
    nav_tv.style = GlyphStyle::Small;
    nav_tv.clear_area = true;
    write!(nav_tv.text, "F1=menu F4=back  ENTER=toggle/edit").unwrap();
    gam.post_textview(&mut nav_tv).expect("can't post footer");

    gam.redraw().expect("can't redraw");
}
