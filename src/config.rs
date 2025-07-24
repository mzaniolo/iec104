struct Config {
	/// The timeout after which the station must acknowledge receipt with
	/// S-frames. The default is 10 seconds.
	t2_timeout: Duration,

	/// The timeout for considering the connection to be non-functional and
	/// close it. The default is 15 seconds. Default is 12
	t1_timeout: Duration,
	k: u32,
	/// Latest acknowledge after receiving w I format APDUs. Default is 8
	w: u32,
}
