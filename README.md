# Timers

A timer suite for [Precursor](https://precursor.dev): a Pomodoro timer, a stopwatch with laps, and a list of named countdowns. All three can run at once, keep running when you leave the app, and alert (vibration and/or a notification) even when another app is in front.

Status: builds at the toolkit's xous-core anchor, clippy-clean, host tests pass (`cd core && cargo test`). **Not yet verified on Renode or hardware**: `capture.py` is written but the Renode capture hasn't been run. Screenshots in `screenshots/` are from version 0.2 and show the old layout.

## Features

- **Pomodoro**: work, short break, long break after N work sessions. The next phase starts by itself with an alert. Progress bar, session count. Work, short and long break are 1 to 180 minutes; sessions before a long break are 1 to 12.
- **Stopwatch**: total time in tenths, current split, up to 99 laps (newest first, scrollable). A lap records a split; the total keeps running.
- **Countdowns**: up to 20 named presets (name up to 20 characters, duration 1 s to 24 h as `SS`, `MM:SS` or `H:MM:SS`). One runs at a time; starting another asks first. Pause, resume, reset. Leaving the countdown screen keeps it running.
- **Background alerts**: while a Pomodoro or countdown is running, the app keeps a 1 s tick even when it isn't in front (drawing is skipped), so the alert fires on time. With nothing armed, the tick stops. The stopwatch alone doesn't tick in the background; its time is computed from timestamps when you come back.
- **Suspend-safe time**: timers are measured from ticktimer `elapsed_ms` timestamps, never by counting ticks. The ticktimer pauses during suspend, so the app also watches the local wall clock (re-synced from the RTC on resume) and adds any gap it finds. If the time zone / clock isn't set, only the ticktimer is used and time spent suspended isn't counted.
- **Home screen** shows what each timer is doing.
- **Settings**: vibrate on alert, notification on alert, Pomodoro lengths and sessions. Settings return to the screen you came from.

## Keys

`∴` opens the system menu and never reaches the app.

| Where | Key | Action |
|---|---|---|
| Everywhere | F1 | Menu for the screen (Up/Down, Enter; F1, F4 or Backspace closes) |
| | F4 / Backspace | Back. On the home screen: leave the app (timers keep running) |
| Home | ↑ ↓ Enter | Choose and open Pomodoro, Stopwatch or Countdowns |
| | `s` | Settings (also on the Pomodoro and Countdowns screens) |
| Pomodoro | Enter / F2 | Start or pause |
| | `r` / F3 | Restart the current phase |
| Stopwatch | Enter / F2 | Start or pause |
| | `l` | Lap |
| | `r` / F3 | Reset (when paused) |
| | ↑ ↓ | Scroll laps |
| Countdowns | Enter | Start the selected countdown, or open it if it's the running one |
| | `n` / `d` | New / delete (asks) |
| Running countdown | Enter / F2 | Start, pause or resume |
| | `r` / F3 | Reset to the full duration (Enter starts it again) |
| | `x` | Stop and clear |
| New countdown | type, Enter | Name, then duration. Invalid input is explained, not dropped. F4 cancels |
| Settings | Enter | Toggle, or set a typed number |
| | digits | Type a value for the selected number |
| | ← → | Adjust the selected number by 1 |
| Confirm | `y` or Enter / `n`, F4 | Yes / No |

## Storage

| PDDB dictionary | Key | Contents |
|---|---|---|
| `timers` | `settings` | JSON: `version`, vibrate, notify, Pomodoro minutes and sessions, countdown presets |

- Written through `precursor-appkit`'s `Store` (delete, rewrite in the key's basis, sync), only when a setting or preset changes.
- Values are checked on load: out-of-range Pomodoro settings are clamped, invalid presets dropped.
- 0.2 binary keys (`pomodoro_settings`, `alert_config`, `countdowns`) are read once, saved as `settings`, then deleted.
- A record that can't be parsed or comes from a newer version isn't overwritten; the status line says changes won't be saved.
- Reloaded when a PDDB basis is locked or unlocked.
- Running timers are not saved: a reboot stops them.

## Limitations

- Renode capture not yet run; no on-device verification of this version.
- Alerts can be up to about 1 s late (1 s tick).
- Setting the clock forward while a timer runs is counted as suspended time.
- No sound (0.2's audio toggle did nothing and was removed).
- Each update redraws the whole screen.

## Layout

```
core/            timer-core: pure logic, no Xous deps, host-tested
  timer.rs       pausable timer from timestamps; stopwatch with laps
  clock.rs       suspend-safe monotonic clock
  pomodoro.rs    phases and validated settings
  countdown.rs   presets, duration parser
  saved.rs       settings record, 0.2 formats
  app.rs         screen state machine, pump interval, alerts
  tests.rs       key-and-clock sequence tests
src/
  main.rs        GAM registration and message loop (precursor-appkit)
  app.rs         clock, pump, storage, alert thread
  ui.rs          drawing
```

## Building

```bash
cd xous-dev-toolkit
bin/integrate --grade /path/to/precursor-timers   # adds the member, merges manifest-fragment.json
cargo xtask renode-image timers                   # from xous-core
```

Host tests: `cd core && cargo test`.

## Changelog

### v0.3.0

- Alerts fire in the background and for timers not on screen; the pump runs while an alert is armed.
- Suspend-safe clock; time always from timestamps.
- Stopwatch lap keeps the total; reset countdown starts again with Enter.
- Pomodoro settings validated (1 to 180 min, 1 to 12 sessions); Settings returns to the previous screen and no longer freezes a running Pomodoro.
- Countdown input errors and the 20-preset limit are explained; names truncated on character boundaries; delete asks first.
- Notifications on their own thread, ticks as blocking scalars (no queued tick bursts).
- F4 at the top leaves the app instead of terminating it.
- Built on `precursor-appkit` (framebuffer canvas, focus handling, versioned JSON storage, basis reloads).

### v0.2.0

- Pomodoro configuration, lap scroll, settings persistence.

### v0.1.0

- Pomodoro, Stopwatch and Countdown modes.

## Author

Made by Tyler Colby, [Colby's Data Movers, LLC](https://colbysdatamovers.com). Issues: [GitHub](https://github.com/tbcolby/precursor-timers/issues).

## License

Apache License, Version 2.0. See [LICENSE](LICENSE).
