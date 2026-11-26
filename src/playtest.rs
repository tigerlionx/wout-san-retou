//! Playtest session log. Every run writes `playtest_logs/session-<time>.csv` next to the game
//! (level changes, every time Ro goes down, and a note each time the tester presses F9), so the
//! testers' sessions can be read afterwards without watching them. Off during autoplay;
//! set WSR_NOLOG=1 to switch it off.

use crate::weapons::Health;
use crate::*;
use std::io::Write;

#[derive(Resource)]
pub struct SessionLog {
    file: Option<std::fs::File>,
    start: bevy::platform::time::Instant,
}

impl SessionLog {
    pub fn open() -> Self {
        // The web build has no folder to write to.
        let off = cfg!(target_arch = "wasm32") || std::env::var_os("WSR_NOLOG").is_some() || std::env::var_os("WSR_AUTOPLAY").is_some();
        let dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("playtest_logs")))
            .unwrap_or_else(|| "playtest_logs".into());
        let file = if off || std::fs::create_dir_all(&dir).is_err() {
            None
        } else {
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            std::fs::File::create(dir.join(format!("session-{stamp}.csv"))).ok()
        };
        let mut log = Self { file, start: bevy::platform::time::Instant::now() };
        log.raw("seconds,event,level,x,z,health,weight,kills,caught,woken,note");
        log
    }

    fn raw(&mut self, line: &str) {
        if let Some(f) = self.file.as_mut() {
            let _ = writeln!(f, "{line}");
            let _ = f.flush();
        }
    }
}

#[derive(bevy::ecs::system::SystemParam)]
pub struct Snapshot<'w, 's> {
    state: Res<'w, State<AppState>>,
    health: Res<'w, Health>,
    weight: Res<'w, Weight>,
    stats: Res<'w, Stats>,
    player: Query<'w, 's, &'static Transform, With<Player>>,
}

impl Snapshot<'_, '_> {
    fn line(&self, log: &SessionLog, event: &str, note: &str) -> String {
        let (x, z) = self.player.iter().next().map(|t| (t.translation.x, t.translation.z)).unwrap_or((0.0, 0.0));
        format!(
            "{:.1},{},{:?},{:.1},{:.1},{:.0},{:.0},{},{},{},{}",
            log.start.elapsed().as_secs_f32(),
            event,
            self.state.get(),
            x,
            z,
            self.health.hp,
            self.weight.0,
            self.stats.kills,
            self.stats.caught,
            self.stats.woken,
            note.replace(',', ";").replace('\n', " ")
        )
    }
}

/// Level changes, deaths, and F9 notes.
pub fn record(
    keys: Res<ButtonInput<KeyCode>>,
    mut log: ResMut<SessionLog>,
    snap: Snapshot,
    mut msg: ResMut<Message>,
    mut prev: Local<Option<(AppState, u32)>>,
) {
    let now = (*snap.state.get(), snap.stats.caught);
    match *prev {
        None => {
            let l = snap.line(&log, "start", "");
            log.raw(&l);
        }
        Some((s, _)) if s != now.0 => {
            let l = snap.line(&log, "level", "");
            log.raw(&l);
        }
        Some((_, c)) if c != now.1 => {
            let l = snap.line(&log, "down", "");
            log.raw(&l);
        }
        _ => {}
    }
    *prev = Some(now);
    if keys.just_pressed(KeyCode::F9) {
        let l = snap.line(&log, "note", "tester pressed F9 (too hard / confusing / boring / great)");
        log.raw(&l);
        msg.0 = "Note saved. Thank you: you can keep playing.".into();
    }
}

/// WSR_FPS=1 prints the average frame rate every 5 seconds (to find scenes that are too heavy).
pub fn fps_report(time: Res<Time>, mut acc: Local<(f32, u32)>) {
    if std::env::var_os("WSR_FPS").is_none() {
        return;
    }
    acc.0 += time.delta_secs();
    acc.1 += 1;
    if acc.0 >= 5.0 {
        println!("[fps] {:.1} fps ({:.1} ms/frame)", acc.1 as f32 / acc.0, 1000.0 * acc.0 / acc.1 as f32);
        *acc = (0.0, 0);
    }
}
