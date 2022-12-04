use anyhow::Context;
use futures::{future::FutureExt, pin_mut, select};
use std::sync::Arc;
use webrtc::api::APIBuilder;
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::MediaEngine;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::track::track_local::track_local_static_rtp::TrackLocalStaticRTP;



pub async fn create_answer(offer: RTCSessionDescription, video_track: Arc<TrackLocalStaticRTP>) -> anyhow::Result<RTCSessionDescription> {
	// Create a MediaEngine object to configure the supported codec
	let mut m = MediaEngine::default();

	m.register_default_codecs()?;

	// Create an InterceptorRegistry. This is the user configurable RTP/RTCP Pipeline.
	// This provides NACKs, RTCP Reports and other features. If you use `webrtc.NewPeerConnection`
	// this is enabled by default. If you are manually managing You MUST create an InterceptorRegistry
	// for each PeerConnection.
	let mut registry = Registry::new();

	// Use the default set of Interceptors
	registry = register_default_interceptors(registry, &mut m)?;

	// Create the API object with the MediaEngine
	let api = APIBuilder::new()
		.with_media_engine(m)
		.with_interceptor_registry(registry)
		.build();

	// Prepare the configuration
	let config = RTCConfiguration {
		ice_servers: vec![RTCIceServer {
			urls: vec!["stun:stun.freeswitch.org:3478".to_owned()],
			..Default::default()
		}],
		..Default::default()
	};

	// Create a new RTCPeerConnection
	let peer_connection = Arc::new(api.new_peer_connection(config).await?);

	// Add this newly created track to the PeerConnection
	let rtp_sender = peer_connection
		.add_track(video_track)
		.await?;

	// Channel to send a signal when the client gets disconnected so we can clean up
	let (disconnected_tx, mut disconnected_rx) = tokio::sync::mpsc::channel::<()>(1);

	tokio::spawn(async move {
		let mut rtcp_buf = vec![0u8; 1500];
		let disconnected_fut = disconnected_rx.recv().fuse();
		pin_mut!(disconnected_fut);
		loop {
			// Read incoming RTCP packets
			// Before these packets are returned they are processed by interceptors.
			// For things like NACK this needs to be called.
			let recv_rtcp_fut = rtp_sender.read(&mut rtcp_buf).fuse();

			pin_mut!(recv_rtcp_fut);
			select! {
				_ = disconnected_fut => {
					println!("Client is disconnected; cleaning up RTP sender");
					rtp_sender.stop().await;
					break;
				},
				// Nothing to actually do with the RTCP packets, but we're supposed to read them, anyway
				_ = recv_rtcp_fut => {}
			}
		}
		anyhow::Result::<()>::Ok(())
	});

	// Set the handler for Peer connection state
	// This will notify you when the peer has connected/disconnected
	peer_connection
		.on_peer_connection_state_change(Box::new(move |s: RTCPeerConnectionState| {
			println!("Peer Connection State has changed: {}", s);

			if s == RTCPeerConnectionState::Failed {
				// Wait until PeerConnection has had no network activity for 30 seconds or another failure. It may be reconnected using an ICE Restart.
				// Use webrtc.PeerConnectionStateDisconnected if you are interested in detecting faster timeout.
				// Note that the PeerConnection may come back from PeerConnectionStateDisconnected.
				println!("Peer Connection state is Failed; cleaning up connection.");
				// Send the disconnected signal so we can clean up the connection objects and stop trying to send data to a connection that's no longer active.
				let _ = disconnected_tx.try_send(());
			}

			Box::pin(async {})
		}));

	// Set the remote SessionDescription
	peer_connection.set_remote_description(offer).await?;

	// Create an answer
	let answer = peer_connection.create_answer(None).await?;

	// Create channel that is blocked until ICE Gathering is complete
	let mut gather_complete = peer_connection.gathering_complete_promise().await;

	// Sets the LocalDescription, and starts our UDP listeners
	peer_connection.set_local_description(answer).await?;

	// Block until ICE Gathering is complete, disabling trickle ICE
	let _ = gather_complete.recv().await;

	// Output the answer
	peer_connection.local_description().await.context("no local description")
}
