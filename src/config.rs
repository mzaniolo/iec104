use std::{path::PathBuf, time::Duration};

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ProtocolConfig {
	/// The period between sending test frames. The default is 20 seconds.
	#[serde(with = "humantime_serde")]
	pub t3: Duration,
	/// The timeout after which the station must acknowledge receipt with
	/// S-frames. The default is 10 seconds.
	#[serde(with = "humantime_serde")]
	pub t2: Duration,
	/// The timeout for considering the connection to be non-functional and
	/// close it. The default is 15 seconds. Default is 12
	#[serde(with = "humantime_serde")]
	pub t1: Duration,
	/// The period for connections attempts. The default is 10 second.
	#[serde(with = "humantime_serde")]
	pub t0: Duration,
	/// Maximum number of sent and unacknowledged ASDUs. Default is 12.
	pub k: u32,
	/// Latest acknowledge after receiving w I format APDUs. Default is 8
	pub w: u32,
}

/// The client TLS configuration
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct TlsClientConfig {
	/// Path to the client key; if not specified, it will be assumed
	/// that the server is configured not to verify client
	/// certificates.
	pub client_key: Option<PathBuf>,
	/// Path to the client certificate; if not specified, it will be
	/// assumed that the server is configured not to verify client
	/// certificates.
	pub client_certificate: Option<PathBuf>,
	/// Path to the server certificate; if not specified, the host's
	/// CA will be used to verify the server.
	pub server_certificate: Option<PathBuf>,
	/// Whether to verify the server's certificates.
	///
	/// This should normally only be used in test environments, as
	/// disabling certificate validation defies the purpose of using
	/// TLS in the first place.
	#[serde(default)]
	pub danger_disable_tls_verify: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ClientConfig {
	/// The address of the server.
	pub address: String,
	/// The port of the server.
	pub port: u16,
	/// The protocol configuration.
	pub protocol: ProtocolConfig,
	/// The TLS configuration.
	pub tls: Option<TlsClientConfig>,
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
