// Copyright (c) The Libra Core Contributors
// SPDX-License-Identifier: Apache-2.0

//! `conn_notifs_channel` is a channel which delivers to the receiver only the last of N
//! messages that might have been sent by sender(s) since the last poll. The items are separated
//! using a key that is provided by the sender with each message.
//!
//! It provides an mpsc channel which has two ends `conn_notifs_channel::Receiver`
//! and `conn_notifs_channel::Sender` which behave similarly to existing mpsc data structures.

use crate::peer_manager::ConnectionNotification;
use channel::{libra_channel, message_queues::QueueStyle};
use libra_types::PeerId;
use std::num::NonZeroUsize;

pub type Sender = libra_channel::Sender<PeerId, ConnectionNotification>;
pub type Receiver = libra_channel::Receiver<PeerId, ConnectionNotification>;

pub fn new() -> (Sender, Receiver) {
    libra_channel::new(QueueStyle::LIFO, NonZeroUsize::new(1).unwrap(), None)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::peer::DisconnectReason;
    use futures::{executor::block_on, future::FutureExt, stream::StreamExt};
    use libra_config::network_id::NetworkContext;
    use libra_network_address::NetworkAddress;

    #[test]
    fn send_n_get_1() {
        let (mut sender, mut receiver) = super::new();
        let peer_id_a = PeerId::random();
        let peer_id_b = PeerId::random();
        let task = async move {
            sender
                .push(
                    peer_id_a,
                    ConnectionNotification::NewPeer(
                        peer_id_a,
                        NetworkAddress::mock(),
                        NetworkContext::mock(),
                    ),
                )
                .unwrap();
            sender
                .push(
                    peer_id_a,
                    ConnectionNotification::LostPeer(
                        peer_id_a,
                        NetworkAddress::mock(),
                        DisconnectReason::ConnectionLost,
                    ),
                )
                .unwrap();
            sender
                .push(
                    peer_id_a,
                    ConnectionNotification::NewPeer(
                        peer_id_a,
                        NetworkAddress::mock(),
                        NetworkContext::mock(),
                    ),
                )
                .unwrap();
            sender
                .push(
                    peer_id_a,
                    ConnectionNotification::LostPeer(
                        peer_id_a,
                        NetworkAddress::mock(),
                        DisconnectReason::Requested,
                    ),
                )
                .unwrap();
            // Ensure that only the last message is received.
            assert_eq!(
                receiver.select_next_some().await,
                ConnectionNotification::LostPeer(
                    peer_id_a,
                    NetworkAddress::mock(),
                    DisconnectReason::Requested
                )
            );
            // Ensures that there is no other value which is ready
            assert_eq!(receiver.select_next_some().now_or_never(), None);

            sender
                .push(
                    peer_id_a,
                    ConnectionNotification::NewPeer(
                        peer_id_a,
                        NetworkAddress::mock(),
                        NetworkContext::mock(),
                    ),
                )
                .unwrap();
            sender
                .push(
                    peer_id_b,
                    ConnectionNotification::NewPeer(
                        peer_id_b,
                        NetworkAddress::mock(),
                        NetworkContext::mock(),
                    ),
                )
                .unwrap();
            // Assert that we receive 2 updates, since they are sent for different peers.
            let _ = receiver.select_next_some().await;
            let _ = receiver.select_next_some().await;
            // Ensures that there is no other value which is ready
            assert_eq!(receiver.select_next_some().now_or_never(), None);
        };
        block_on(task);
    }
}
