//! Timers for Precursor: Pomodoro, stopwatch and countdowns.
//!
//! Timer rules live in `timer-core` (host-tested). This crate registers with
//! GAM, owns the clock and the pump, raises alerts and draws.

mod app;
mod ui;

use num_traits::{FromPrimitive, ToPrimitive};
use precursor_appkit::{app as kit, keys, pump, store};

/// xous-names server name. Any unique string; NOT the GAM app name.
const SERVER_NAME: &str = "_Timers_";

#[derive(Debug, num_derive::FromPrimitive, num_derive::ToPrimitive)]
enum AppOp {
    Redraw = 0,
    Rawkeys,
    FocusChange,
    /// Blocking scalar from the pump; answered with `pump::ack`.
    Tick,
    BasisChanged,
    Quit,
}

fn main() -> ! {
    log_server::init_wait().unwrap();
    log::set_max_level(log::LevelFilter::Info);
    log::info!("timers PID is {}", xous::process::id());

    let xns = xous_names::XousNames::new().unwrap();
    let sid = xns.register_name(SERVER_NAME, None).expect("can't register server");

    // GAM app name comes from apps/manifest.json key "timers".
    let reg = kit::register(
        &xns,
        sid,
        gam::APP_NAME_TIMERS,
        gam::UxType::Framebuffer,
        kit::Opcodes {
            redraw: AppOp::Redraw.to_u32().unwrap(),
            rawkeys: Some(AppOp::Rawkeys.to_u32().unwrap()),
            focus_change: Some(AppOp::FocusChange.to_u32().unwrap()),
            got_input: None,
        },
    );

    let pump = pump::Pump::spawn(xous::connect(sid).unwrap(), AppOp::Tick.to_usize().unwrap(), 1000);
    let store = store::Store::open(timer_core::saved::DICT);
    store::watch_bases(xous::connect(sid).unwrap(), AppOp::BasisChanged.to_usize().unwrap());
    let mut app = app::App::new(reg.screen, store, pump);

    loop {
        let msg = xous::receive_message(sid).unwrap();
        match FromPrimitive::from_usize(msg.body.id()) {
            Some(AppOp::Redraw) => app.draw(),
            Some(AppOp::Rawkeys) => xous::msg_scalar_unpack!(msg, k1, k2, k3, k4, {
                for key in keys::decode_rawkeys([k1, k2, k3, k4]) {
                    if app.on_key(key) {
                        kit::leave(app.screen());
                    }
                }
                app.draw();
            }),
            Some(AppOp::FocusChange) => xous::msg_scalar_unpack!(msg, code, _, _, _, {
                app.set_foreground(matches!(kit::focus_state(code), gam::FocusState::Foreground));
            }),
            Some(AppOp::Tick) => {
                app.tick();
                pump::ack(&msg);
            }
            Some(AppOp::BasisChanged) => app.reload(),
            Some(AppOp::Quit) => break,
            None => log::warn!("unknown opcode {:?}", msg.body.id()),
        }
    }
    app.quit();
    kit::shutdown(&xns, sid)
}
