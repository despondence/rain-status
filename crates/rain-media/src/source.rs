use crate::session::MediaSession;
use tokio::sync::mpsc;

#[derive(Clone, Debug)]
pub enum SourceUpdate {
    Upsert(MediaSession),
    Remove(String),
}

pub trait MediaSource: Send + Sync {
    fn name(&self) -> &'static str;
    fn start(&self, sink: mpsc::Sender<SourceUpdate>) -> tokio::task::JoinHandle<()>;
}
