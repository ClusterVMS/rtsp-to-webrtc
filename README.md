# RTSP To WebRTC

Converts an RTSP stream from an IP camera to a WebRTC stream.

Uses a slight variation of WebRTC-HTTP Ingestion Protocol (WHIP) to exchange SDP offers and answers (WHIP has the client streaming to the server, while we have the server stream to the client).

This application is primarily intended for use with ClusterVMS, but can also be used stand-alone.



## Usage

* Either build the docker container, or build locally with `cargo build --release`. Building in release mode is HIGHLY recommended, as performance is MUCH worse under debug mode.
* Run with these environment variables:
	* `CAM_SRC_URL`: URL of the camera stream to connect to (do not include username or password in URL)
	* `CAM_USERNAME`: username to use to connect to the camera stream
	* `CAM_PASSWORD`: password to use to connect to the camera stream
* If using containers, forward port 8000
* Open `example.html` in your browser on the same machine



## License

Licensed under either of the Apache License, Version 2.0 or the MIT License at your option. Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this crate by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

