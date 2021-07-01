use libp2p::identify::{Identify, IdentifyConfig, IdentifyEvent};
use libp2p::kad::{record::store::MemoryStore, Kademlia, KademliaConfig, KademliaEvent};
use libp2p::ping::{Ping, PingConfig, PingEvent};
use libp2p::relay::v2::relay;
use libp2p::{identity, Multiaddr, NetworkBehaviour, PeerId};
use std::str::FromStr;
use std::time::Duration;

const BOOTNODES: [&str; 4] = [
    "QmNnooDu7bfjPFoTZYxMNLWUQJyrVwtbZg5gBMjTezGAJN",
    "QmQCU2EcMqAqQPR2i9bChDtGNJchTbq5TbXJJ16u19uLTa",
    "QmbLHAnMoJPWSCR5Zhtx6BHJX9KiKNN6tpvbUcqanj75Nb",
    "QmcZf59bWwK5XFi76CZX8cbJ4BhTzzA3gU1ZjYZcYW3dwt",
];

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "Event", event_process = false)]
pub struct Behaviour {
    relay: relay::Relay,
    ping: Ping,
    identify: Identify,
    pub kademlia: Kademlia<MemoryStore>,
}

impl Behaviour {
    pub fn new(pub_key: identity::PublicKey) -> Self {
        let kademlia = {
            let mut kademlia_config = KademliaConfig::default();
            // Instantly remove records and provider records.
            //
            // TODO: Replace hack with option to disable both.
            kademlia_config.set_record_ttl(Some(Duration::from_secs(0)));
            kademlia_config.set_provider_record_ttl(Some(Duration::from_secs(0)));
            let mut kademlia = Kademlia::with_config(
                pub_key.clone().into_peer_id(),
                MemoryStore::new(pub_key.clone().into_peer_id()),
                kademlia_config,
            );
            let bootaddr = Multiaddr::from_str("/dnsaddr/bootstrap.libp2p.io").unwrap();
            for peer in &BOOTNODES {
                kademlia.add_address(&PeerId::from_str(peer).unwrap(), bootaddr.clone());
            }
            kademlia.bootstrap().unwrap();
            kademlia
        };

        Self {
            relay: relay::Relay::new(PeerId::from(pub_key.clone()), Default::default()),
            ping: Ping::new(PingConfig::new()),
            identify: Identify::new(
                IdentifyConfig::new("ipfs/0.1.0".to_string(), pub_key).with_agent_version(format!(
                    "rust-libp2p-server/{}",
                    env!("CARGO_PKG_VERSION")
                )),
            ),
            kademlia,
        }
    }
}

#[derive(Debug)]
pub enum Event {
    Ping(PingEvent),
    Identify(Box<IdentifyEvent>),
    Relay(relay::Event),
    Kademlia(KademliaEvent),
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

impl From<relay::Event> for Event {
    fn from(event: relay::Event) -> Self {
        Event::Relay(event)
    }
}

impl From<KademliaEvent> for Event {
    fn from(event: KademliaEvent) -> Self {
        Event::Kademlia(event)
    }
}
