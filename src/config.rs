use time::Duration;

struct Config {
	/// The period between sending test frames. The default is 20 seconds.
	t3: Duration,
	/// The timeout after which the station must acknowledge receipt with
	/// S-frames. The default is 10 seconds.
	t2: Duration,
	/// The timeout for considering the connection to be non-functional and
	/// close it. The default is 15 seconds. Default is 12
	t1: Duration,
	/// The period for connections attempts. The default is 10 second.
	t0: Duration,
	/// Maximum number of sent and unacknowledged ASDUs. Default is 12.
	k: u32,
	/// Latest acknowledge after receiving w I format APDUs. Default is 8
	w: u32,
}
