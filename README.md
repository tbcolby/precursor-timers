# Timers

A unified timer suite for the [Precursor](https://www.crowdsupply.com/sutajio-kosagi/precursor) device with three modes: Pomodoro Timer, Stopwatch, and Countdown Collection.

![Mode Select](screenshots/mode_select.png)

## Features

### Pomodoro Timer

Classic Pomodoro Technique timer with configurable work/break intervals.

- **25-minute work sessions** (configurable via PDDB)
- **5-minute short breaks** between sessions
- **15-minute long break** after 4 cycles
- Auto-transitions between work and break phases
- Progress bar showing time elapsed in current phase
- Session counter tracking completed work sessions
- Vibration and notification alerts on phase transitions

![Pomodoro](screenshots/pomodoro.png)

**Controls:**
| Key | Action |
|-----|--------|
| Enter | Start / Pause |
| r | Reset current phase |
| s | Open settings |
| q | Back to mode select |

### Stopwatch

Precision stopwatch with centisecond display and lap recording.

- **HH:MM:SS.cs** format (centisecond precision)
- Display updates every 100ms while running
- Record up to 99 laps (most recent shown first)
- Scrollable lap list
- Lap times show individual split durations

![Stopwatch](screenshots/stopwatch.png)

**Controls:**
| Key | Action |
|-----|--------|
| Enter | Start / Pause |
| l | Record lap (while running) |
| r | Reset (while stopped) |
| q | Back to mode select |

### Countdown Collection

Create and manage named countdown timers.

- Store up to 20 named timers
- Enter duration in MM:SS format
- Progress bar during countdown
- Vibration and notification on expiry
- Persisted to PDDB (survives app restart)

**Controls (list):**
| Key | Action |
|-----|--------|
| Enter | Start selected timer |
| n | Create new timer |
| d | Delete selected timer |
| Up/Down | Navigate list |
| q | Back to mode select |

**Controls (running):**
| Key | Action |
|-----|--------|
| Enter | Pause / Resume |
| r | Reset to original duration |
| q | Back to timer list |

### Settings

Configure alert behavior for timer expirations.

![Settings](screenshots/settings.png)

| Setting | Default | Description |
|---------|---------|-------------|
| Vibration | ON | Device vibration on timer events |
| Notification | ON | Modal notification popup |
| Audio | OFF | Audio tone (requires codec) |

## Installation

Clone into your [xous-core](https://github.com/betrusted-io/xous-core) apps directory:

```bash
cd xous-core/apps
git clone https://github.com/tbcolby/precursor-timers.git timers
```

Register the app in your workspace. Add to `xous-core/Cargo.toml` members:

```toml
"apps/timers",
"apps/timers/timer-core",
```

Register in `apps/manifest.json`:

```json
"timers": {
    "context_name": "Timers",
    "menu_name": {
        "appmenu.timers": {
            "en": "Timers",
            "en-tts": "Timers"
        }
    }
}
```

Build and run:

```bash
cargo xtask renode-image timers
```

## Architecture

```
timer-core/             Pure timing logic (no Xous deps, host-testable)
src/
  main.rs               Event loop, state machine, pump thread
  pomodoro.rs           Pomodoro state (work/break cycles)
  stopwatch.rs          Stopwatch state (laps)
  countdown.rs          Named countdown timers
  storage.rs            PDDB persistence
  alerts.rs             Vibration/notification alerts
  ui.rs                 Drawing functions per screen
```

### timer-core Library

The `timer-core` crate provides platform-independent timing logic, testable on the host:

- `TimerCore` struct: start/pause/reset/lap with millisecond precision
- Count-up mode (stopwatch) and count-down mode (countdown/pomodoro)
- Time formatting: `format_hms`, `format_hms_cs`, `format_ms`
- Binary serialization helpers for PDDB storage

Run tests: `cargo test -p timer-core`

### Pump Thread

A dedicated background thread sends periodic `Pump` messages to the main event loop:

- **100ms** interval for stopwatch (centisecond display)
- **1000ms** interval for pomodoro/countdown (second display)
- Automatically stopped when app loses focus or timers are paused
- Zero CPU usage when no timer is actively running

### PDDB Storage

All persistent data stored in the `timers` dictionary:

| Key | Format | Content |
|-----|--------|---------|
| `pomodoro_settings` | 25 bytes | work_ms + short_ms + long_ms + cycles |
| `alert_config` | 3 bytes | vibration + audio + notification flags |
| `countdowns` | variable | count + [name_len + name + duration_ms]... |

## Testing

```bash
# Run timer-core unit tests on host
cargo test -p timer-core

# Build for Renode emulation
cargo xtask renode-image timers
```

## License

Licensed under the Apache License, Version 2.0.
