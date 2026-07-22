//! Background game-state polling.
//!
//! All game-memory reads (inventory scans, event flags, subregion) and the view-model build are
//! expensive and used to run inline in the hudhook `render` callback — i.e. on the game's DX12
//! Present thread. Even throttled to every 250 ms, that produced a periodic frame hitch (a stall
//! several times per second), which players perceive as stutter.
//!
//! This worker moves that work onto a dedicated thread. It owns the [`GameStateReader`] and the
//! [`ChallengeTracker`], polls at [`POLL_INTERVAL`], and publishes freshly built
//! [`OverlayViewModel`]s over a channel. The render thread only drains the channel (a cheap move)
//! and draws the latest snapshot, so game memory work never blocks Present again.

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use er_game_state::{GameStateReader, GameStateSource};
use er_overlay_common::{
    BossPanelScope, ChallengeConfig, ChallengeSnapshot, ChallengeTracker, PbDirection,
};
use er_overlay_ui::{build_view_model, resolve_metric_count, OverlayViewModel};

/// How often the worker polls the game and republishes the view model.
///
/// Runs off the render thread, so this cadence is independent of the game's frame rate and does
/// not affect Present latency.
const POLL_INTERVAL: Duration = Duration::from_millis(250);

/// Everything the worker needs to build a view model that is derived from the config + layout
/// (as opposed to live game memory). The render thread recomputes and pushes this whenever the
/// config or layout changes; the worker caches the latest copy.
#[derive(Clone)]
pub struct PollInputs {
    pub data_refs: Vec<String>,
    pub equipped_refs: HashSet<String>,
    pub historic_refs: HashSet<String>,
    pub boss_panel_scope: BossPanelScope,
    pub checks_panel_scope: BossPanelScope,
    pub challenge_config: ChallengeConfig,
    pub pb_source: String,
    pub pb_mode: PbDirection,
    pub start_flag: u32,
}

enum Command {
    Inputs(Box<PollInputs>),
    InvalidateBossCache,
}

/// Render-thread handle to the polling worker: publishes input changes and drains the latest
/// view model. Joins the worker on drop.
pub struct PollWorker {
    cmd_tx: Sender<Command>,
    // `Receiver` is `Send` but not `Sync`; hudhook requires the render loop to be `Sync`. Only the
    // render thread ever touches it, so the mutex is uncontended.
    vm_rx: Mutex<Receiver<OverlayViewModel>>,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl PollWorker {
    pub fn spawn(reader: GameStateReader, challenge: ChallengeTracker, inputs: PollInputs) -> Self {
        let (cmd_tx, cmd_rx) = std::sync::mpsc::channel::<Command>();
        let (vm_tx, vm_rx) = std::sync::mpsc::channel::<OverlayViewModel>();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_thread = Arc::clone(&stop);

        let handle = thread::Builder::new()
            .name("er_overlay_poll".into())
            .spawn(move || {
                run(reader, challenge, inputs, cmd_rx, vm_tx, stop_thread);
            })
            .ok();

        Self {
            cmd_tx,
            vm_rx: Mutex::new(vm_rx),
            stop,
            handle,
        }
    }

    /// Applies the most recently published view model, if any. Cheap enough to call every frame.
    /// Returns `true` when a newer view model was consumed.
    pub fn drain_into(&self, vm: &mut OverlayViewModel) -> bool {
        let Ok(rx) = self.vm_rx.lock() else {
            return false;
        };
        let mut updated = false;
        while let Ok(new) = rx.try_recv() {
            *vm = new;
            updated = true;
        }
        updated
    }

    /// Pushes config/layout-derived inputs to the worker (applied on its next tick).
    pub fn set_inputs(&self, inputs: PollInputs) {
        let _ = self.cmd_tx.send(Command::Inputs(Box::new(inputs)));
    }

    /// Drops the cached boss kill count so it is recomputed after a boss-table reload.
    pub fn invalidate_boss_cache(&self) {
        let _ = self.cmd_tx.send(Command::InvalidateBossCache);
    }
}

impl Drop for PollWorker {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn run(
    mut reader: GameStateReader,
    mut challenge: ChallengeTracker,
    mut inputs: PollInputs,
    cmd_rx: Receiver<Command>,
    vm_tx: Sender<OverlayViewModel>,
    stop: Arc<AtomicBool>,
) {
    challenge.sync_config(&inputs.challenge_config);

    while !stop.load(Ordering::Relaxed) {
        loop {
            match cmd_rx.try_recv() {
                Ok(Command::Inputs(new)) => {
                    challenge.sync_config(&new.challenge_config);
                    inputs = *new;
                }
                Ok(Command::InvalidateBossCache) => reader.invalidate_boss_cache(),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return,
            }
        }

        reader.poll();
        if reader.is_ready() {
            let vm = build(&reader, &mut challenge, &inputs);
            if vm_tx.send(vm).is_err() {
                return;
            }
        }

        thread::sleep(POLL_INTERVAL);
    }
}

/// Builds the view model and folds in the challenge snapshot, mirroring the pre-refactor
/// `OverlayApp::refresh_view_model` logic (now off the render thread).
fn build(
    reader: &GameStateReader,
    challenge: &mut ChallengeTracker,
    inputs: &PollInputs,
) -> OverlayViewModel {
    // Build first (with a placeholder challenge snapshot) so the PB metric can be resolved from
    // live game data before the challenge tracker consumes it.
    let mut vm = build_view_model(
        reader,
        &inputs.data_refs,
        &inputs.equipped_refs,
        &inputs.historic_refs,
        inputs.boss_panel_scope,
        inputs.checks_panel_scope,
        ChallengeSnapshot::default(),
    );

    let snapshot = if inputs.challenge_config.enabled && reader.challenge_update_ready() {
        challenge.configure(
            &inputs.pb_source,
            inputs.pb_mode,
            &inputs.challenge_config.budget_metric,
            inputs.challenge_config.budget_max,
        );
        let budget_value = resolve_metric_count(&inputs.challenge_config.budget_metric, &vm);
        let pb_value = resolve_metric_count(&inputs.pb_source, &vm);
        challenge.update(budget_value, pb_value, reader.get_flag(inputs.start_flag))
    } else {
        challenge.snapshot()
    };
    challenge.flush();
    vm.challenge = snapshot;
    vm
}
