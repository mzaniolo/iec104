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
	/// Maximum number of sent and unacknowledged ASDUs. Default is 12.
	#[serde(default = "default_number::<12>")]
	pub k: u16,
	/// Latest acknowledge after receiving w I format APDUs. Default is 8
	#[serde(default = "default_number::<8>")]
	pub w: u16,
	/// The originator address for the IEC 104 connection.
	pub originator_address: u8,
}

/// The client TLS configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TlsConfig {
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
pub struct LinkConfig {
	/// The address of the server.
	pub address: String,
	/// The port of the server.
	pub port: u16,
	/// Server or Client mode.
	pub server: bool,
	/// The protocol configuration.
	#[serde(default)]
	pub protocol: ProtocolConfig,
	/// The TLS configuration.
	#[serde(default)]
	pub tls: Option<TlsConfig>,
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
			originator_address: 1,
		}
	}
}

impl Default for LinkConfig {
	fn default() -> Self {
		Self {
			address: "127.0.0.1".to_owned(),
			port: 2404,
			server: false,
			protocol: ProtocolConfig::default(),
			tls: None,
		}
	}
}

const fn default_number<const N: u16>() -> u16 {
	N
}

const fn default_duration<const N: u64>() -> Duration {
	Duration::from_secs(N)
}
