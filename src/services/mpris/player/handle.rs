use std::sync::Arc;

use crate::services::mpris::proxy::MediaPlayer2PlayerProxy;

use super::{Player, PlayerMonitor};

/// Player handle containing the reactive model and associated resources.
pub(crate) struct PlayerHandle {
    pub(crate) player: Arc<Player>,
    pub(crate) proxy: MediaPlayer2PlayerProxy<'static>,
    pub(crate) _monitor: PlayerMonitor,
}
