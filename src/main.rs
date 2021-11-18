use futures::executor::block_on;
use futures::stream::StreamExt;
use futures_timer::Delay;
use libp2p::core::identity::ed25519;
use libp2p::core::upgrade;
use libp2p::dns;
use libp2p::identify::{IdentifyEvent, IdentifyInfo};
use libp2p::kad;
use libp2p::metrics::{Metrics, Recorder};
use libp2p::noise;
use libp2p::swarm::{SwarmBuilder, SwarmEvent};
use libp2p::tcp::TcpConfig;
use libp2p::Transport;
use libp2p::{identity, PeerId};
use log::{debug, info};
use open_metrics_client::metrics::info::Info;
use open_metrics_client::registry::Registry;
use std::error::Error;
use std::path::PathBuf;
use std::str::FromStr;
use std::task::Poll;
use std::thread;
use std::time::Duration;
use structopt::StructOpt;
use zeroize::Zeroizing;

mod behaviour;
mod config;
mod metric_server;

const BOOTSTRAP_INTERVAL: Duration = Duration::from_secs(5 * 60);

#[derive(Debug, StructOpt)]
#[structopt(name = "libp2p server", about = "A rust-libp2p server binary.")]
struct Opt {
    /// Path to IPFS config file.
    #[structopt(long)]
    config: Option<PathBuf>,

    /// Metric endpoint path.
    #[structopt(long, default_value = "/metrics")]
    metrics_path: String,

    /// Whether to run the libp2p Kademlia protocol and join the IPFS DHT.
    #[structopt(long)]
    enable_kamdelia: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let opt = Opt::from_args();

    let (local_peer_id, local_keypair) = match &opt.config {
        Some(path) => {
            let config = Zeroizing::new(config::Config::from_file(path.as_path())?);

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

    let behaviour = behaviour::Behaviour::new(local_keypair.public(), opt.enable_kamdelia);
    let mut swarm = SwarmBuilder::new(transport, behaviour, local_peer_id)
        .executor(Box::new(|fut| {
            async_std::task::spawn(fut);
        }))
        .build();
    swarm.listen_on("/ip4/0.0.0.0/tcp/4001".parse()?)?;

    let mut metric_registry = Registry::default();
    let metrics = Metrics::new(&mut metric_registry);
    let build_info = Info::new(vec![("version".to_string(), env!("CARGO_PKG_VERSION"))]);
    metric_registry.register(
        "build",
        "A metric with a constant '1' value labeled by version",
        Box::new(build_info),
    );
    thread::spawn(move || block_on(metric_server::run(metric_registry, opt.metrics_path)));

    let mut bootstrap_timer = Delay::new(BOOTSTRAP_INTERVAL);

    block_on(async {
        loop {
            if let Poll::Ready(()) = futures::poll!(&mut bootstrap_timer) {
                bootstrap_timer.reset(BOOTSTRAP_INTERVAL);
                let _ = swarm
                    .behaviour_mut()
                    .kademlia
                    .as_mut()
                    .map(|k| k.bootstrap());
            }

            match swarm.next().await.expect("Swarm not to terminate.") {
                SwarmEvent::Behaviour(behaviour::Event::Identify(e)) => {
                    info!("{:?}", e);
                    metrics.record(&*e);

                    if let IdentifyEvent::Received {
                        peer_id,
                        info:
                            IdentifyInfo {
                                listen_addrs,
                                protocols,
                                ..
                            },
                    } = *e
                    {
                        if protocols
                            .iter()
                            .any(|p| p.as_bytes() == kad::protocol::DEFAULT_PROTO_NAME)
                        {
                            for addr in listen_addrs {
                                swarm
                                    .behaviour_mut()
                                    .kademlia
                                    .as_mut()
                                    .map(|k| k.add_address(&peer_id, addr));
                            }
                        }
                    }
                }
                SwarmEvent::Behaviour(behaviour::Event::Ping(e)) => {
                    debug!("{:?}", e);
                    metrics.record(&e);
                }
                SwarmEvent::Behaviour(behaviour::Event::Kademlia(e)) => {
                    debug!("{:?}", e);
                    metrics.record(&e);
                }
                SwarmEvent::Behaviour(behaviour::Event::Relay(e)) => {
                    info!("{:?}", e);
                    metrics.record(&e)
                }
                e => {
                    if let SwarmEvent::NewListenAddr { address, .. } = &e {
                        println!("Listening on {:?}", address);
                    }

                    metrics.record(&e)
                }
            }
        }
    })
}
