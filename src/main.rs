use futures::executor::block_on;
use libp2p::core::identity::ed25519;
use libp2p::core::upgrade;
use libp2p::dns;
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
use std::str::FromStr;
use std::thread;
use structopt::StructOpt;
use zeroize::Zeroizing;

mod behaviour;
mod config;
mod metric_server;

#[derive(Debug, StructOpt)]
#[structopt(name = "libp2p server", about = "A rust-libp2p server binary.")]
struct Opt {
    /// Path to IPFS config file.
    #[structopt(long)]
    config: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let opt = Opt::from_args();

    let (local_peer_id, local_keypair) = match opt.config {
        Some(path) => {
            let config = Zeroizing::new(config::Config::from_file(path)?);

            let keypair = identity::Keypair::from_protobuf_encoding(&Zeroizing::new(
                base64::decode(config.identity.priv_key.as_bytes())?,
            ))?;

            let peer_id = keypair.public().into();
            assert_eq!(
                    PeerId::from_str(&config.identity.peer_id)?,
                    peer_id,
                    "Expect peer id derived from private key and peer id retrieved from config to match."
                );

            (peer_id, keypair)
        }
        None => {
            let keypair = identity::Keypair::Ed25519(ed25519::Keypair::generate());
            (keypair.public().into(), keypair)
        }
    };
    println!("Local peer id: {:?}", local_peer_id);

    let transport = TcpConfig::new();
    let transport = block_on(dns::DnsConfig::system(transport)).unwrap();
    let noise_keys = noise::Keypair::<noise::X25519Spec>::new()
        .into_authentic(&local_keypair)
        .expect("Signing libp2p-noise static DH keypair failed.");
    let transport = transport
        .upgrade(upgrade::Version::V1)
        .authenticate(noise::NoiseConfig::xx(noise_keys).into_authenticated())
        .multiplex(libp2p::yamux::YamuxConfig::default())
        .boxed();

    let behaviour = behaviour::Behaviour::new(local_keypair.public());
    let mut swarm = Swarm::new(transport, behaviour, local_peer_id);
    swarm.listen_on("/ip4/0.0.0.0/tcp/4001".parse()?)?;

    let mut metric_registry = Registry::default();
    let metrics = Metrics::new(&mut metric_registry);
    thread::spawn(move || block_on(metric_server::run(metric_registry)));

    block_on(async {
        loop {
            match swarm.next_event().await {
                SwarmEvent::Behaviour(behaviour::Event::Identify(e)) => {
                    info!("{:?}", e);
                    metrics.record(&*e);
                }
                SwarmEvent::Behaviour(behaviour::Event::Ping(e)) => {
                    debug!("{:?}", e);
                    metrics.record(&e);
                }
                SwarmEvent::Behaviour(behaviour::Event::Kademlia(e)) => {
                    debug!("{:?}", e);
                    metrics.record(&e);
                }
                SwarmEvent::Behaviour(behaviour::Event::Relay(e)) => info!("{:?}", e),
                e => {
                    if let SwarmEvent::NewListenAddr(addr) = &e {
                        println!("Listening on {:?}", addr);
                    }

                    metrics.record(&e)
                }
            }
        }
    })
}
