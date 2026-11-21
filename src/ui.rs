//! Drawing. Sizes come from the canvas; nothing assumes 336×536.

use gam::menu::*;
use gam::GlyphStyle;
use precursor_appkit::layout::{rows_that_fit, ListState};
use precursor_appkit::screen::{Screen, MARGIN};
use timer_core::format::{format_remaining, format_tenths};
use timer_core::{ScreenId, TimerCore, TimerState, Timers};

fn list(screen: &Screen, top: isize, bottom: isize, items: &[String], selected: usize) {
    let row_h = screen.line_height(GlyphStyle::Regular) + 4;
    let visible = rows_that_fit(bottom - top, row_h).max(1);
    let state = ListState { selected, top: if selected >= visible { selected + 1 - visible } else { 0 } };
    screen.list(top, bottom, items, &state);
}

fn footer_for(t: &Timers) -> &'static str {
    if t.confirm_text().is_some() {
        return "y yes   n / F4 no";
    }
    if t.menu().is_some() {
        return "↑↓ choose  Enter select  F1/F4 close";
    }
    match t.screen() {
        ScreenId::Home => "Enter open  s settings  F1 menu  F4 leave",
        ScreenId::Pomodoro => "Enter start/pause  r restart  s settings  F4 back",
        ScreenId::Stopwatch => "Enter start/pause  l lap  r reset  F4 back",
        ScreenId::Countdowns => "Enter start  n new  d delete  F4 back",
        ScreenId::CountdownRun => "Enter start/pause  r reset  x stop  F4 back",
        ScreenId::NewName => "type  Enter next  F4 cancel",
        ScreenId::NewDuration => "type  Enter add  F4 cancel",
        ScreenId::Settings => "↑↓ choose  Enter toggle/set  ←→ adjust  F4 back",
        ScreenId::Help => "F4 / Backspace close",
    }
}

fn title_for(t: &Timers) -> &'static str {
    match t.screen() {
        ScreenId::Home => "Timers",
        ScreenId::Pomodoro => "Pomodoro",
        ScreenId::Stopwatch => "Stopwatch",
        ScreenId::Countdowns | ScreenId::CountdownRun => "Countdowns",
        ScreenId::NewName | ScreenId::NewDuration => "New countdown",
        ScreenId::Settings => "Settings",
        ScreenId::Help => "Help",
    }
}

fn state_label(t: &TimerCore) -> &'static str {
    match t.state() {
        TimerState::Running => "Running",
        TimerState::Paused => "Paused",
        TimerState::Stopped => "Ready: Enter starts",
    }
}

fn progress_bar(screen: &Screen, y: isize, fraction: f32) {
    let (l, r) = (MARGIN + 10, screen.size().x - MARGIN - 10);
    screen.frame(Point::new(l, y), Point::new(r, y + 14), 1);
    let fill = ((r - l - 4) as f32 * fraction.clamp(0.0, 1.0)) as isize;
    if fill > 0 {
        screen.fill(Point::new(l + 2, y + 2), Point::new(l + 2 + fill, y + 12), PixelColor::Dark);
    }
}

/// One centred line; returns the y below it.
fn centered(screen: &Screen, y: isize, style: GlyphStyle, text: &str) -> isize {
    let h = screen.line_height(style);
    screen.text_centered(Point::new(MARGIN, y), Point::new(screen.size().x - MARGIN, y + h), style, text);
    y + h
}

pub fn draw(screen: &Screen, t: &Timers, now: u64) {
    screen.clear();
    let top = screen.header(title_for(t));
    let mut bottom = screen.footer(footer_for(t));
    let width = screen.size().x;
    let small = screen.line_height(GlyphStyle::Small);
    if !t.status().is_empty() {
        bottom -= small * 2 + 4;
        screen.text_in(Point::new(MARGIN, bottom), Point::new(width - MARGIN, bottom + small * 2 + 2), GlyphStyle::Small, t.status());
    }

    match t.screen() {
        ScreenId::Home => list(screen, top, bottom, &t.home_rows(now), t.home_selected()),
        ScreenId::Pomodoro => {
            let p = &t.pomodoro;
            let mut y = centered(screen, top + 16, GlyphStyle::Bold, p.phase().label()) + 12;
            y = centered(screen, y, GlyphStyle::Large, &format_remaining(p.timer.remaining_ms(now).unwrap_or(0))) + 14;
            progress_bar(screen, y, p.timer.progress(now));
            y += 30;
            let s = p.settings();
            y = centered(screen, y, GlyphStyle::Regular, &format!("Session {} of {}", (p.cycle() + 1).min(s.cycles), s.cycles)) + 4;
            y = centered(screen, y, GlyphStyle::Regular, &format!("{} done since start", p.completed())) + 4;
            centered(screen, y, GlyphStyle::Small, state_label(&p.timer));
        }
        ScreenId::Stopwatch => {
            let (total, split) = t.stopwatch_text(now);
            let mut y = centered(screen, top + 16, GlyphStyle::Large, &total) + 8;
            let laps = t.stopwatch.laps();
            y = centered(screen, y, GlyphStyle::Regular, &format!("Lap {}  {split}", laps.len() + 1)) + 4;
            y = centered(screen, y, GlyphStyle::Small, state_label(&t.stopwatch.timer)) + 10;
            if !laps.is_empty() {
                screen.line(Point::new(MARGIN, y), Point::new(width - MARGIN, y));
                y += 6;
                let row = screen.line_height(GlyphStyle::Regular) + 2;
                let rows = rows_that_fit(bottom - y, row);
                // Newest first, scrolled back by lap_scroll.
                for (k, i) in (0..laps.len()).rev().skip(t.lap_scroll()).take(rows).enumerate() {
                    let text = format!("Lap {:>2}   {}", i + 1, format_tenths(laps[i]));
                    screen.text_in(Point::new(MARGIN + 10, y + k as isize * row), Point::new(width - MARGIN, y + (k as isize + 1) * row), GlyphStyle::Regular, &text);
                }
            }
        }
        ScreenId::Countdowns => {
            let rows = t.countdown_rows(now);
            if rows.is_empty() {
                screen.text_in(Point::new(MARGIN, top + 10), Point::new(width - MARGIN, bottom), GlyphStyle::Regular, "No countdowns yet. Press n to add one.");
            } else {
                list(screen, top, bottom, &rows, t.countdown_selected());
            }
        }
        ScreenId::CountdownRun => {
            if let (Some(name), Some(timer)) = (t.countdowns.active_name(), t.countdowns.active()) {
                let mut y = centered(screen, top + 16, GlyphStyle::Bold, name) + 12;
                y = centered(screen, y, GlyphStyle::Large, &format_remaining(timer.remaining_ms(now).unwrap_or(0))) + 14;
                progress_bar(screen, y, timer.progress(now));
                centered(screen, y + 30, GlyphStyle::Small, state_label(timer));
            }
        }
        ScreenId::NewName => {
            let text = format!("Name (up to 20 characters):\n\n{}_", t.input());
            screen.text_in(Point::new(MARGIN, top + 10), Point::new(width - MARGIN, bottom), GlyphStyle::Regular, &text);
        }
        ScreenId::NewDuration => {
            let text = format!("Duration for \"{}\"\nSS, MM:SS or H:MM:SS (up to 24 h):\n\n{}_", t.pending_name(), t.input());
            screen.text_in(Point::new(MARGIN, top + 10), Point::new(width - MARGIN, bottom), GlyphStyle::Regular, &text);
        }
        ScreenId::Settings => list(screen, top, bottom, &t.settings_rows(), t.settings_selected()),
        ScreenId::Help => {
            screen.text_in(Point::new(MARGIN, top + 4), Point::new(width - MARGIN, bottom), GlyphStyle::Regular, t.help_text());
        }
    }

    if let Some(menu) = t.menu() {
        let items: Vec<String> = menu.labels().iter().map(|s| s.to_string()).collect();
        let h = (screen.line_height(GlyphStyle::Regular) + 4) * items.len() as isize + 12;
        let tl = Point::new(MARGIN + 20, top + 10);
        let br = Point::new(width - MARGIN - 20, (top + 10 + h).min(bottom));
        screen.fill(tl, br, PixelColor::Light);
        list(screen, tl.y + 4, br.y - 4, &items, menu.selected);
        screen.frame(tl, br, 2);
    }
    if let Some(text) = t.confirm_text() {
        let tl = Point::new(MARGIN + 10, top + 40);
        let br = Point::new(width - MARGIN - 10, top + 130);
        screen.fill(tl, br, PixelColor::Light);
        screen.frame(tl, br, 2);
        screen.text_in(Point::new(tl.x + 10, tl.y + 10), Point::new(br.x - 10, br.y - small - 12), GlyphStyle::Regular, text);
        screen.text_in(Point::new(tl.x + 10, br.y - small - 8), Point::new(br.x - 10, br.y - 4), GlyphStyle::Small, "y yes   n / F4 no");
    }
    screen.flush();
}
