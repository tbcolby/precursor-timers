"""Renode capture sequence for Timers (not yet run).

Run from xous-dev-toolkit:
    python3 scripts/renode_capture.py --app timers --script ../precursor-timers/capture.py

Contract: run(ctl, outdir) -> number of failed steps. The app is already launched.
Keys: Enter start/pause, F4 or Backspace back, F1 menu, s settings,
l lap, n new countdown.
"""
import os


def run(ctl, outdir) -> int:
    failures = 0

    def shot(name):
        nonlocal failures
        if not ctl.try_screenshot(os.path.join(outdir, name)):
            failures += 1

    ctl.sleep(2.0)
    shot("mode_select.png")
    ctl.inject_line("")                 # Pomodoro
    ctl.inject_line("")                 # start
    ctl.sleep(3.0)
    shot("pomodoro.png")
    ctl.inject_key("s")                 # settings (returns to Pomodoro)
    ctl.sleep(1.5)
    shot("settings.png")
    ctl.timed_key("F4", after=1.0)      # back to Pomodoro, still running
    ctl.timed_key("F4", after=1.0)      # home shows the running Pomodoro
    shot("home_running.png")
    ctl.timed_key("Down", after=0.7)
    ctl.inject_line("")                 # Stopwatch
    ctl.inject_line("")                 # start
    ctl.sleep(2.0)
    ctl.inject_key("l")
    ctl.sleep(1.5)
    ctl.inject_key("l")
    ctl.sleep(1.0)
    shot("stopwatch.png")
    ctl.inject_line("")                 # pause
    ctl.timed_key("F4", after=1.0)
    ctl.timed_key("Down", after=0.7)
    ctl.inject_line("")                 # Countdowns
    ctl.inject_key("n")
    ctl.sleep(0.5)
    ctl.inject_line("Tea")
    ctl.sleep(0.5)
    shot("countdown_new.png")
    ctl.inject_line("")                 # accept 05:00
    ctl.sleep(1.0)
    shot("countdown_list.png")
    ctl.inject_line("")                 # start Tea
    ctl.sleep(2.0)
    shot("countdown_run.png")
    ctl.timed_key("F4", after=1.0)      # list (Tea keeps running)
    ctl.timed_key("F4", after=1.0)      # home
    ctl.timed_key("F4", after=1.5)      # leave the app; timers keep running
    shot("left_app.png")
    return failures
