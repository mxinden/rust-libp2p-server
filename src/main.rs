use futures::executor::block_on;
use futures::stream::StreamExt;
use libp2p::core::upgrade;
use libp2p::identify::{Identify, IdentifyEvent, IdentifyConfig};
use libp2p::noise;
use libp2p::ping::{Ping, PingEvent, PingConfig};
use libp2p::relay::v2::{Relay, RelayEvent};
use libp2p::tcp::TcpConfig;
use libp2p::Transport;
use libp2p::{identity, NetworkBehaviour, PeerId, Swarm};
use std::error::Error;
use std::task::{Context, Poll};

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    println!("Local peer id: {:?}", local_peer_id);

    let tcp_transport = TcpConfig::new();

    let noise_keys = noise::Keypair::<noise::X25519Spec>::new()
        .into_authentic(&local_key)
        .expect("Signing libp2p-noise static DH keypair failed.");

    let transport = tcp_transport
        .upgrade(upgrade::Version::V1)
        .authenticate(noise::NoiseConfig::xx(noise_keys).into_authenticated())
        .multiplex(libp2p::yamux::YamuxConfig::default())
        .boxed();

    #[derive(NetworkBehaviour)]
    #[behaviour(out_event = "Event", event_process = false)]
    struct Behaviour {
        relay: Relay,
        ping: Ping,
        identify: Identify,
    }

    #[derive(Debug)]
    enum Event {
        Ping(PingEvent),
        Identify(IdentifyEvent),
        Relay(RelayEvent),
    }

    impl From<PingEvent> for Event {
        fn from(e: PingEvent) -> Self {
            Event::Ping(e)
        }
    }

    impl From<IdentifyEvent> for Event {
        fn from(e: IdentifyEvent) -> Self {
            Event::Identify(e)
        }
    }

    impl From<RelayEvent> for Event {
        fn from(e: RelayEvent) -> Self {
            Event::Relay(e)
        }
    }

    let behaviour = Behaviour {
        relay: Relay::new(local_peer_id, Default::default()),
        ping: Ping::new(PingConfig::new()),
        identify: Identify::new(IdentifyConfig::new(
            "/TODO/0.0.1".to_string(),
            local_key.public(),
        )),
    };

    let mut swarm = Swarm::new(transport, behaviour, local_peer_id);

    // Listen on all interfaces and whatever port the OS assigns
    swarm.listen_on("/ip4/0.0.0.0/tcp/4001".parse()?)?;

    let mut listening = false;
    block_on(futures::future::poll_fn(move |cx: &mut Context<'_>| {
        loop {
            match swarm.poll_next_unpin(cx) {
                Poll::Ready(Some(Event::Relay(event))) => println!("{:?}", event),
                Poll::Ready(Some(_)) => {},
                Poll::Ready(None) => return Poll::Ready(Ok(())),
                Poll::Pending => {
                    if !listening {
                        for addr in Swarm::listeners(&swarm) {
                            println!("Listening on {:?}", addr);
                            listening = true;
                        }
                    }
                    break;
                }
            }
        }
        Poll::Pending
    }))
}
