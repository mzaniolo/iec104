use std::{path::PathBuf, time::Duration};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProtocolConfig {
	/// The period between sending test frames. The default is 20 seconds.
	#[serde(with = "humantime_serde", default = "default_duration::<20>")]
	pub t3: Duration,
	/// The timeout after which the station must acknowledge receipt with
	/// S-frames. The default is 10 seconds.
	#[serde(with = "humantime_serde", default = "default_duration::<10>")]
	pub t2: Duration,
	/// The timeout for considering the connection to be non-functional and
	/// close it. The default is 15 seconds.
	#[serde(with = "humantime_serde", default = "default_duration::<15>")]
	pub t1: Duration,
	/// The period for connections attempts. The default is 10 second.
	#[serde(with = "humantime_serde", default = "default_duration::<10>")]
	pub t0: Duration,
	/// Maximum number of sent and unacknowledged I-format APDUs (`k` in IEC
	/// 60870-5-104). Default is 12.
	///
	/// On the **server**, outgoing ASDUs beyond this limit are queued per
	/// connection until the peer acknowledges with I/S frames (up to
	/// [`Self::max_pending_outgoing_asdu`]). On the **client**,
	/// [`crate::Client::send_asdu`] fails with `OutputBufferFull` while the `k`
	/// window is full instead of enqueueing.
	#[serde(default = "default_number::<12>")]
	pub k: u16,
	/// Latest acknowledge after receiving w I format APDUs. Default is 8
	#[serde(default = "default_number::<8>")]
	pub w: u16,
	/// Maximum ASDUs queued per connection waiting for `k` window space (server
	/// and client receive-handler). When full,
	/// [`crate::server::ConnectionHandler::send_asdu`] and
	/// [`crate::Client::send_asdu`] fail so upper layers can back off. Default
	/// **1024**. Use **0** for no limit (not recommended on untrusted peers).
	#[serde(default = "default_max_pending_outgoing_asdu")]
	pub max_pending_outgoing_asdu: u32,
	/// The originator address for the IEC 104 connection.
	pub originator_address: u8,
}

impl ProtocolConfig {
	/// `None` if [`Self::max_pending_outgoing_asdu`] is `0` (unlimited pending
	/// queue).
	#[must_use]
	pub fn max_pending_outgoing_asdu_limit(&self) -> Option<usize> {
		(self.max_pending_outgoing_asdu != 0).then_some(self.max_pending_outgoing_asdu as usize)
	}
}

/// The client TLS configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TlsClientConfig {
	/// Path to the client key; if not specified, it will be assumed
	/// that the server is configured not to verify client
	/// certificates.
	#[serde(default)]
	pub client_key: Option<PathBuf>,
	/// Path to the client certificate; if not specified, it will be
	/// assumed that the server is configured not to verify client
	/// certificates.
	#[serde(default)]
	pub client_certificate: Option<PathBuf>,
	/// Path to the server certificate; if not specified, the host's
	/// CA will be used to verify the server.
	#[serde(default)]
	pub server_certificate: Option<PathBuf>,
	/// Whether to verify the server's certificates.
	///
	/// This should normally only be used in test environments, as
	/// disabling certificate validation defies the purpose of using
	/// TLS in the first place.
	#[serde(default)]
	pub danger_disable_tls_verify: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClientConfig {
	/// The address of the server.
	pub address: String,
	/// The port of the server.
	pub port: u16,
	/// The protocol configuration.
	#[serde(default)]
	pub protocol: ProtocolConfig,
	/// The TLS configuration.
	#[serde(default)]
	pub tls: Option<TlsClientConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServerConfig {
	/// The address of the server.
	pub address: String,
	/// The port of the server.
	pub port: u16,
	#[serde(default)]
	pub protocol: ProtocolConfig,
	/// The TLS configuration.
	#[serde(default)]
	pub tls: Option<TlsServerConfig>,
}

/// The server TLS configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TlsServerConfig {
	/// Path to the server certificate.
	pub server_certificate: PathBuf,
	/// Path to the server key
	pub server_key: PathBuf,
}

impl Default for ProtocolConfig {
	fn default() -> Self {
		Self {
			t3: Duration::from_secs(20),
			t2: Duration::from_secs(10),
			t1: Duration::from_secs(12),
			t0: Duration::from_secs(10),
			k: 12,
			w: 8,
			max_pending_outgoing_asdu: 1024,
			originator_address: 1,
		}
	}
}

impl Default for ClientConfig {
	fn default() -> Self {
		Self {
			address: "127.0.0.1".to_owned(),
			port: 2404,
			protocol: ProtocolConfig::default(),
			tls: None,
		}
	}
}

impl Default for ServerConfig {
	fn default() -> Self {
		Self {
			address: "127.0.0.1".to_owned(),
			port: 2404,
			protocol: ProtocolConfig::default(),
			tls: None,
		}
	}
}

const fn default_number<const N: u16>() -> u16 {
	N
}

const fn default_max_pending_outgoing_asdu() -> u32 {
	1024
}

const fn default_duration<const N: u64>() -> Duration {
	Duration::from_secs(N)
}
