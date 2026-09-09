#![allow(unused_imports)]

mod bandwidth_logging;
mod bandwidth_metrics;
mod behaviour;
mod build;
mod dns;
mod identity;
mod other_transport;
mod provider;
mod quic;
mod relay;
mod swarm;
mod tcp;
mod websocket;

use bandwidth_logging::*;
use bandwidth_metrics::*;
pub use behaviour::BehaviourError;
use behaviour::*;
use build::*;
use dns::*;
use libp2p_core::{muxing::StreamMuxerBox, Transport};
use libp2p_identity::Keypair;
pub use other_transport::TransportError;
use other_transport::*;
use provider::*;
use quic::*;
use relay::*;
use swarm::*;
use tcp::*;
#[cfg(all(not(target_arch = "wasm32"), feature = "websocket"))]
pub use websocket::WebsocketError;
use websocket::*;

use super::SwarmBuilder;

// Only transports with separate security and multiplexing upgrades use these helpers.
#[cfg(any(
    feature = "relay",
    all(
        not(target_arch = "wasm32"),
        any(feature = "tokio", feature = "async-std"),
        any(feature = "tcp", feature = "websocket"),
    ),
))]
mod upgrade;
#[cfg(any(
    feature = "relay",
    all(
        not(target_arch = "wasm32"),
        any(feature = "tokio", feature = "async-std"),
        any(feature = "tcp", feature = "websocket"),
    ),
))]
use upgrade::*;

pub trait AuthenticatedMultiplexedTransport:
    Transport<
        Error = Self::E,
        Dial = Self::D,
        ListenerUpgrade = Self::U,
        Output = (libp2p_identity::PeerId, StreamMuxerBox),
    > + Send
    + Unpin
    + 'static
{
    type E: Send + Sync + 'static;
    type D: Send;
    type U: Send;
}

impl<T> AuthenticatedMultiplexedTransport for T
where
    T: Transport<Output = (libp2p_identity::PeerId, StreamMuxerBox)> + Send + Unpin + 'static,
    <T as Transport>::Error: Send + Sync + 'static,
    <T as Transport>::Dial: Send,
    <T as Transport>::ListenerUpgrade: Send,
{
    type E = T::Error;
    type D = T::Dial;
    type U = T::ListenerUpgrade;
}
