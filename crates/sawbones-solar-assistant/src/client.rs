//! A read-only MQTT connection.
//!
//! # Why this type exists
//!
//! Solar Assistant accepts setting changes over MQTT: a message sent to a
//! topic ending in `/set` or `/set_verified` can change how an inverter
//! charges, discharges or shuts down. This crate must never do that, and
//! "we were careful" is not good enough.
//!
//! [`ReadOnlyMqtt`] owns the underlying `rumqttc` client and event loop in
//! private fields and exposes exactly two operations on them:
//! [`ReadOnlyMqtt::subscribe`] and [`ReadOnlyMqtt::poll`]. There is no method
//! that sends an application message, no accessor that hands out the inner
//! client, and no `Deref`. Code outside this module therefore *cannot* send
//! to any topic, because it has nothing to send with. Inside this module
//! the inner client's sending methods are never called; a test
//! (`tests/read_only.rs`) scans the crate's source to keep it that way.
//!
//! Three further choices close the indirect routes:
//!
//! - **No last will.** A last-will message is one the broker sends on a
//!   client's behalf. The connect options are built here, privately, and no
//!   will is ever set.
//! - **QoS 0 subscriptions.** The broker delivers at most once, so this
//!   client never owes it an acknowledgement packet.
//! - **A clean session.** Nothing about this client is kept by the broker
//!   after it leaves.
//!
//! What the client does put on the wire: the MQTT connect packet, subscribe
//! requests, and keep-alive pings. Nothing else.
//!
//! This is a property of the code, not of the broker. If Solar Assistant's
//! "Allow setting changes" option is off (its documented default), the unit
//! also refuses writes from everyone; leave it off.

use std::fmt;
use std::time::Duration;

use rumqttc::{AsyncClient, Event, EventLoop, MqttOptions, Packet, QoS, SubscribeReasonCode};
use thiserror::Error;

/// Largest inbound MQTT packet accepted. Discovery configs are the big ones.
const MAX_INBOUND_PACKET_BYTES: usize = 256 * 1024;

/// Largest outbound MQTT packet. Only connect and subscribe packets are ever
/// sent, and they are small.
const MAX_OUTBOUND_PACKET_BYTES: usize = 4 * 1024;

/// A username and password for the broker.
///
/// The `Debug` output hides both values.
#[derive(Clone)]
pub struct Credentials {
    username: String,
    password: String,
}

impl Credentials {
    /// Wraps a username and password.
    pub fn new(username: String, password: String) -> Self {
        Self { username, password }
    }
}

impl fmt::Debug for Credentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Credentials(<redacted>)")
    }
}

/// How to reach the broker.
#[derive(Debug, Clone)]
pub struct ConnectParams {
    /// Broker host name or address.
    pub host: String,
    /// Broker TCP port. Solar Assistant's examples use 1883.
    pub port: u16,
    /// MQTT client id. Must be unique on the broker.
    pub client_id: String,
    /// Optional username and password.
    pub credentials: Option<Credentials>,
    /// Keep-alive interval.
    pub keep_alive: Duration,
}

/// An error from the read-only client.
#[derive(Debug, Error)]
pub enum ClientError {
    /// The connection failed or dropped. Calling [`ReadOnlyMqtt::poll`]
    /// again starts a new connection attempt.
    #[error("MQTT connection error: {0}")]
    Connection(String),
    /// A subscribe request could not be queued.
    #[error("could not queue MQTT subscribe request: {0}")]
    Subscribe(String),
    /// A subscription filter was empty.
    #[error("subscription filter is empty")]
    EmptyFilter,
}

/// Something that arrived from the broker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Inbound {
    /// The broker accepted the connection. Subscriptions do not survive a
    /// reconnect (the session is clean), so subscribe again on each of these.
    Connected,
    /// The broker answered a subscribe request. `granted` is false if it
    /// refused any filter in the request.
    SubscribeAck {
        /// Whether every filter in the request was granted.
        granted: bool,
    },
    /// An application message.
    Message {
        /// The topic it arrived on.
        topic: String,
        /// The payload bytes.
        payload: Vec<u8>,
        /// Whether the broker flagged it as retained.
        retained: bool,
    },
    /// Any other protocol event (pings and the like). Nothing to do.
    Other,
}

/// An MQTT connection that can subscribe and receive, and nothing else.
/// See the [module documentation](self) for why.
pub struct ReadOnlyMqtt {
    client: AsyncClient,
    eventloop: EventLoop,
}

impl ReadOnlyMqtt {
    /// Prepares a connection. Nothing is sent until the first
    /// [`ReadOnlyMqtt::poll`].
    pub fn new(params: ConnectParams) -> Self {
        let mut options = MqttOptions::new(params.client_id, params.host, params.port);
        options.set_keep_alive(params.keep_alive.max(Duration::from_secs(5)));
        options.set_clean_session(true);
        options.set_max_packet_size(MAX_INBOUND_PACKET_BYTES, MAX_OUTBOUND_PACKET_BYTES);
        if let Some(c) = params.credentials {
            options.set_credentials(c.username, c.password);
        }
        // No will is set here, and `options` never leaves this function.
        let (client, eventloop) = AsyncClient::new(options, 16);
        Self { client, eventloop }
    }

    /// Asks the broker for messages matching `filter`, at QoS 0.
    pub async fn subscribe(&self, filter: &str) -> Result<(), ClientError> {
        if filter.is_empty() {
            return Err(ClientError::EmptyFilter);
        }
        self.client
            .subscribe(filter, QoS::AtMostOnce)
            .await
            .map_err(|e| ClientError::Subscribe(e.to_string()))
    }

    /// Waits for the next thing from the broker. On an error the connection
    /// is down; wait (see [`crate::backoff::Backoff`]) and call again to
    /// reconnect.
    pub async fn poll(&mut self) -> Result<Inbound, ClientError> {
        match self.eventloop.poll().await {
            Ok(Event::Incoming(Packet::ConnAck(_))) => Ok(Inbound::Connected),
            Ok(Event::Incoming(Packet::SubAck(ack))) => Ok(Inbound::SubscribeAck {
                granted: !ack
                    .return_codes
                    .iter()
                    .any(|c| matches!(c, SubscribeReasonCode::Failure)),
            }),
            Ok(Event::Incoming(Packet::Publish(p))) => Ok(Inbound::Message {
                topic: p.topic,
                payload: p.payload.to_vec(),
                retained: p.retain,
            }),
            Ok(_) => Ok(Inbound::Other),
            Err(e) => Err(ClientError::Connection(e.to_string())),
        }
    }
}

impl fmt::Debug for ReadOnlyMqtt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ReadOnlyMqtt { .. }")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credentials_do_not_leak_through_debug() {
        let c = Credentials::new("operator".to_owned(), "hunter2".to_owned());
        let shown = format!("{c:?}");
        assert!(!shown.contains("operator"));
        assert!(!shown.contains("hunter2"));
        let p = ConnectParams {
            host: "solar-assistant.local".to_owned(),
            port: 1883,
            client_id: "test".to_owned(),
            credentials: Some(c),
            keep_alive: Duration::from_secs(30),
        };
        assert!(!format!("{p:?}").contains("hunter2"));
    }

    #[tokio::test]
    async fn an_unreachable_broker_is_an_error_not_a_panic() {
        // Port 1 on the loopback address: nothing listens there, so the
        // attempt is refused locally. No real broker is involved.
        let mut c = ReadOnlyMqtt::new(ConnectParams {
            host: "127.0.0.1".to_owned(),
            port: 1,
            client_id: "sawbones-test".to_owned(),
            credentials: None,
            keep_alive: Duration::from_secs(5),
        });
        assert!(matches!(c.poll().await, Err(ClientError::Connection(_))));
        // And it can be polled again (this is what a reconnect is).
        assert!(matches!(c.poll().await, Err(ClientError::Connection(_))));
    }

    #[tokio::test]
    async fn an_empty_filter_is_refused() {
        let c = ReadOnlyMqtt::new(ConnectParams {
            host: "127.0.0.1".to_owned(),
            port: 1,
            client_id: "sawbones-test".to_owned(),
            credentials: None,
            keep_alive: Duration::from_secs(5),
        });
        assert!(matches!(
            c.subscribe("").await,
            Err(ClientError::EmptyFilter)
        ));
    }
}
