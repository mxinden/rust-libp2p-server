use futures::executor::block_on;
use futures::stream::StreamExt;
use libp2p::core::upgrade;
use libp2p::identify::{Identify, IdentifyConfig, IdentifyEvent};
use libp2p::metrics::{Metrics, Recorder};
use libp2p::noise;
use libp2p::ping::{Ping, PingConfig, PingEvent};
use libp2p::relay::v2::{Relay, RelayEvent};
use libp2p::swarm::{NetworkBehaviourAction, NetworkBehaviourEventProcess, PollParameters};
use libp2p::tcp::TcpConfig;
use libp2p::Transport;
use libp2p::{identity, NetworkBehaviour, PeerId, Swarm};
use log::{debug, info};
use open_metrics_client::registry::Registry;
use std::collections::VecDeque;
use std::error::Error;
use std::task::{Context, Poll};

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "Event", poll_method = "poll")]
pub struct Behaviour {
    relay: Relay,
    ping: Ping,
    identify: Identify,

    #[behaviour(ignore)]
    metrics: Metrics,
    #[behaviour(ignore)]
    event_buffer: VecDeque<Event>,
}

impl Behaviour {
    pub fn new(pub_key: identity::PublicKey, metric_registry: &mut Registry) -> Self {
        Self {
            relay: Relay::new(PeerId::from(pub_key.clone()), Default::default()),
            ping: Ping::new(PingConfig::new()),
            identify: Identify::new(IdentifyConfig::new("/TODO/0.0.1".to_string(), pub_key)),
            metrics: Metrics::new(metric_registry),
            event_buffer: Default::default(),
        }
    }

    fn poll<TEv>(
        &mut self,
        _: &mut Context,
        _: &mut impl PollParameters,
    ) -> Poll<NetworkBehaviourAction<TEv, Event>> {
        if let Some(event) = self.event_buffer.pop_front() {
            return Poll::Ready(NetworkBehaviourAction::GenerateEvent(event));
        }

        Poll::Pending
    }
}

#[derive(Debug)]
pub enum Event {
    Ping(PingEvent),
    Identify(IdentifyEvent),
    Relay(RelayEvent),
}

impl NetworkBehaviourEventProcess<PingEvent> for Behaviour {
    fn inject_event(&mut self, event: PingEvent) {
        self.metrics.record(&event);
        self.event_buffer.push_back(Event::Ping(event));
    }
}

impl NetworkBehaviourEventProcess<IdentifyEvent> for Behaviour {
    fn inject_event(&mut self, event: IdentifyEvent) {
        // self.metrics.record(&event);
        self.event_buffer.push_back(Event::Identify(event));
    }
}

impl NetworkBehaviourEventProcess<RelayEvent> for Behaviour {
    fn inject_event(&mut self, event: RelayEvent) {
        // self.metrics.record(&event);
        self.event_buffer.push_back(Event::Relay(event));
    }
}
