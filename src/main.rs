use futures::executor::block_on;
use futures::stream::StreamExt;
use libp2p::core::upgrade;
use libp2p::identity::ed25519;
use libp2p::noise;
use libp2p::tcp::TcpConfig;
use libp2p::Transport;
use libp2p::{identity, PeerId, Swarm};
use log::{debug, info};
use open_metrics_client::registry::Registry;
use std::error::Error;
use std::path::PathBuf;
use std::task::{Context, Poll};
use std::thread;
use structopt::StructOpt;

mod behaviour;
mod metric_server;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "Kademlia exporter",
    about = "Monitor the state of a Kademlia Dht."
)]
struct Opt {
    #[structopt(long)]
    identity: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let opt = Opt::from_args();

    let local_key: identity::Keypair = if let Some(path) = opt.identity {
        let mut bytes = hex::decode(std::fs::read_to_string(path)?)?;
        let secret_key = ed25519::SecretKey::from_bytes(&mut bytes)?;
        identity::Keypair::Ed25519(secret_key.into())
    } else {
        identity::Keypair::generate_ed25519()
    };
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

    let mut metric_registry = Registry::default();

    let behaviour = behaviour::Behaviour::new(local_key.public(), &mut metric_registry);
    let mut swarm = Swarm::new(transport, behaviour, local_peer_id);
    swarm.listen_on("/ip4/0.0.0.0/tcp/4001".parse()?)?;

    thread::spawn(move || block_on(metric_server::run(metric_registry)));

    let mut listening = false;
    block_on(futures::future::poll_fn(move |cx: &mut Context<'_>| {
        loop {
            match swarm.poll_next_unpin(cx) {
                Poll::Ready(Some(behaviour::Event::Relay(event))) => info!("{:?}", event),
                Poll::Ready(Some(event)) => debug!("{:?}", event),
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
