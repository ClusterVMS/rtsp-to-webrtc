# RTSP To WebRTC

Converts an RTSP stream from an IP camera to a WebRTC stream.

Uses a slight variation of WebRTC-HTTP Ingestion Protocol (WHIP) to exchange SDP offers and answers (WHIP has the client streaming to the server, while we have the server stream to the client).

This application is primarily intended for use with ClusterVMS, but can also be used stand-alone.



## Usage

* Either build the docker container, or build locally with `cargo build --release`. Building in release mode is HIGHLY recommended, as performance is MUCH worse under debug mode.
* Create TOML files describing the cameras and streams you want forwarded. See `sample-config.toml` for format example.
	* When sharing config files between several ClusterVMS components, it's recommended to keep the login details and other sensitive config in separate files, accessible only to the applications that need them.
* Run the executable, pointing it to your config files
	* E.g. `./rtsp-to-webrtc -c my-config.toml -c my-secret-config.toml`
* If using containers, forward port 8000
* Open `example.html` in your browser on the same machine



## License

Licensed under either of the Apache License, Version 2.0 or the MIT License at your option. Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this crate by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

