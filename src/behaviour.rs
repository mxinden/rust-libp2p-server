use libp2p::identify::{Identify, IdentifyConfig, IdentifyEvent};
use libp2p::ping::{Ping, PingConfig, PingEvent};
use libp2p::relay::v2::{Relay, RelayEvent};
use libp2p::{identity, NetworkBehaviour, PeerId};

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "Event", event_process = false)]
pub struct Behaviour {
    relay: Relay,
    ping: Ping,
    identify: Identify,
}

impl Behaviour {
    pub fn new(pub_key: identity::PublicKey) -> Self {
        Self {
            relay: Relay::new(PeerId::from(pub_key.clone()), Default::default()),
            ping: Ping::new(PingConfig::new()),
            identify: Identify::new(IdentifyConfig::new("/TODO/0.0.1".to_string(), pub_key)),
        }
    }
}

#[derive(Debug)]
pub enum Event {
    Ping(PingEvent),
    Identify(Box<IdentifyEvent>),
    Relay(RelayEvent),
}

impl From<PingEvent> for Event {
    fn from(event: PingEvent) -> Self {
        Event::Ping(event)
    }
}

impl From<IdentifyEvent> for Event {
    fn from(event: IdentifyEvent) -> Self {
        Event::Identify(Box::new(event))
    }
}

impl From<RelayEvent> for Event {
    fn from(event: RelayEvent) -> Self {
        Event::Relay(event)
    }
}
