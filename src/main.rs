use futures::executor::block_on;
use libp2p::core::upgrade;
use libp2p::identity::ed25519;
use libp2p::metrics::{Metrics, Recorder};
use libp2p::noise;
use libp2p::swarm::SwarmEvent;
use libp2p::tcp::TcpConfig;
use libp2p::Transport;
use libp2p::{identity, PeerId, Swarm};
use log::{debug, info};
use open_metrics_client::registry::Registry;
use std::error::Error;
use std::path::PathBuf;
use std::thread;
use structopt::StructOpt;

mod behaviour;
mod metric_server;

#[derive(Debug, StructOpt)]
#[structopt(name = "libp2p relay server", about = "Relay libp2p connections.")]
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

    let behaviour = behaviour::Behaviour::new(local_key.public());
    let mut swarm = Swarm::new(transport, behaviour, local_peer_id);
    swarm.listen_on("/ip4/0.0.0.0/tcp/4001".parse()?)?;

    let mut metric_registry = Registry::default();
    let metrics = Metrics::new(&mut metric_registry);
    thread::spawn(move || block_on(metric_server::run(metric_registry)));

    let mut listening = false;
    block_on(async {
        loop {
            match swarm.next_event().await {
                SwarmEvent::Behaviour(behaviour::Event::Ping(e)) => {
                    debug!("{:?}", e);
                    metrics.record(&e)
                }
                SwarmEvent::Behaviour(behaviour::Event::Relay(e)) => info!("{:?}", e),
                SwarmEvent::Behaviour(_) => {}
                e => metrics.record(&e),
            }

            if !listening {
                for addr in Swarm::listeners(&swarm) {
                    println!("Listening on {:?}", addr);
                    listening = true;
                }
            }
        }
    })
}
