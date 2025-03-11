
use yage_net::{event_loop::{Registry, EventLoop}, notifier::Notifier};

pub(crate) struct NetHandle {
  registry: Registry,
}
