use async_std::task;
use clap::{Arg, ArgAction, Command};
use clustervms::config;
use clustervms::StreamId;
use core::time::Duration;
use futures::StreamExt;
use log::error;
use retina::client::PacketItem;
use rocket::{Request, Response};
use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::Header;
use std::collections::HashMap;
use std::io::Write;
use std::pin::Pin;
use std::sync::Arc;
use webrtc::api::media_engine::{MIME_TYPE_H264};
use webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecCapability;
use webrtc::track::track_local::{TrackLocalWriter};
use webrtc::track::track_local::track_local_static_rtp::TrackLocalStaticRTP;

#[macro_use] extern crate rocket;

mod common;
mod rest_api;
mod webrtc_utils;

use crate::common::VideoTrackMap;

// Since the UI is served by another server, we may need to setup CORS to allow the UI to make requests to this server.
pub struct CORS;

#[rocket::async_trait]
impl Fairing for CORS {
	fn info(&self) -> Info {
		Info {
			name: "Add CORS headers to responses",
			kind: Kind::Response
		}
	}

	async fn on_response<'r>(&self, _request: &'r Request<'_>, response: &mut Response<'r>) {
		response.set_header(Header::new("Access-Control-Allow-Origin", "*"));
		response.set_header(Header::new("Access-Control-Allow-Methods", "POST, GET, PATCH, OPTIONS"));
		response.set_header(Header::new("Access-Control-Allow-Headers", "*"));
		response.set_header(Header::new("Access-Control-Allow-Credentials", "true"));
	}
}



// Originally copied from https://github.com/webrtc-rs/webrtc/tree/master/examples/examples/rtp-to-webrtc
#[rocket::main]
async fn main() -> anyhow::Result<()> {
	let mut app = Command::new("rtsp-to-webrtc")
		.version("0.2.3")
		.author("Alicrow")
		.about("Forwards an RTSP stream as a WebRTC stream.")
		.arg(
			Arg::new("config")
				.action(ArgAction::Append)	// Allow argument to be specified multiple times
				.short('c')
				.long("config")
				.help("TOML file with ClusterVMS config")
		)
		.arg(
			Arg::new("FULLHELP")
				.help("Prints more detailed help information")
				.long("fullhelp"),
		)
		.arg(
			Arg::new("debug")
				.long("debug")
				.short('d')
				.help("Prints debug log information"),
		);

	let matches = app.clone().get_matches();

	if matches.contains_id("FULLHELP") {
		app.print_long_help().unwrap();
		std::process::exit(0);
	}

	let debug = matches.contains_id("debug");
	if debug {
		env_logger::Builder::new()
			.format(|buf, record| {
				writeln!(
					buf,
					"{}:{} [{}] {} - {}",
					record.file().unwrap_or("unknown"),
					record.line().unwrap_or(0),
					record.level(),
					chrono::Local::now().format("%H:%M:%S.%6f"),
					record.args()
				)
			})
			.filter(None, log::LevelFilter::Trace)
			.init();
	}

	let mut config_manager = config::ConfigManager::new();

	let config_filename_matches = matches.get_many::<String>("config");
	match config_filename_matches {
		Some(filenames) => {
			config_manager.read_config(filenames.map(|v| v.as_str()).collect())?;
		},
		None => {
			// Use default file path
			config_manager.read_default_config_files()?;
		}
	};

	let mut video_tracks = VideoTrackMap::new();

	for (camera_id, camera_info) in &config_manager.get_config().cameras {
		let mut streams_for_camera = HashMap::<StreamId, Arc<TrackLocalStaticRTP>>::new();
		for (stream_id, stream_info) in &camera_info.streams {
			let stream_settings = common::StreamSettings {
				username: camera_info.username.clone().unwrap_or_default().clone(),
				password: camera_info.password.clone().unwrap_or_default().clone(),
				source_url: stream_info.source_url.clone(),
			};
			match create_video_track(stream_settings).await {
				Ok(video_track) => {
					streams_for_camera.insert(stream_id.clone(), video_track.clone());
				}
				Err(e) => {
					error!("Could not create track for camera {camera_id} stream {stream_id} video stream due to error: {e:?}");
					// TODO: keep trying periodically
				}
			}
		}
		video_tracks.insert(camera_id.clone(), streams_for_camera);
	}

	let _rocket = rocket::build()
		.attach(rest_api::stage(video_tracks))
		.attach(CORS)
		.launch()
		.await?;

	anyhow::Ok(())
}

async fn create_video_track(stream_settings: common::StreamSettings) -> anyhow::Result<Arc<TrackLocalStaticRTP>> {
	// Create Track that we send video back to client on
	let video_track = Arc::new(TrackLocalStaticRTP::new(
		RTCRtpCodecCapability {
			mime_type: MIME_TYPE_H264.to_owned(),
			..Default::default()
		},
		"video".to_owned(),
		"webrtc-rs".to_owned(),
	));

	// Set up RTSP connection to camera

	let video_track_clone = video_track.clone();

	// Thread that reads from the input stream and writes packets to the output streams
	tokio::spawn(async move {
		loop {
			let session_options = retina::client::SessionOptions::default().creds(Some(retina::client::Credentials {username: stream_settings.username.clone(), password: stream_settings.password.clone()}) );
			let session = match retina::client::Session::describe(stream_settings.source_url.clone(), session_options).await {
				Ok(mut session) => {
					let video_i = session
						.streams()
						.iter()
						.position(|s| s.media() == "video" && s.encoding_name() == "h264")
						.ok_or_else(|| error!("Could not find H.264 video stream")).unwrap();
					match session.setup(video_i, retina::client::SetupOptions::default()).await {
						Ok(_) => session.play(retina::client::PlayOptions::default()).await,
						Err(e) => Err(e)
					}
				}
				Err(e) => Err(e)
			};

			match session {
				Ok(mut session) => {
					// Read RTP packets forever and send them to the WebRTC Client
					'read_loop: loop {
						match Pin::new(&mut session).next().await {
							None => {
								error!("Source RTSP stream returned None; The stream must have closed.");
								break 'read_loop;
							}
							Some(Err(e)) => {
								error!("error while reading input stream: {e}");
								// FIXME: keep track of whether we're connected or not
								break 'read_loop;
							}
							Some(Ok(PacketItem::Rtp(packet))) => {
								let raw_rtp = packet.raw();
								if let Err(err) = video_track_clone.write(&raw_rtp).await {
									if webrtc::Error::ErrClosedPipe == err {
										// The peerConnection has been closed.
										// FIXME: when would this even occur?
									} else {
										println!("video_track write err: {}", err);
									}
								}
							}
							Some(Ok(PacketItem::Rtcp(_))) => {
								// Do nothing with RTCP packets for now
							}
							Some(Ok(something)) => {
								error!("Received something that we can't handle; it was {:?}", something);
							}
						}
					}
				}
				Err(e) => {
					error!("Failed to connect to input stream, error: {e}");
				}
			}

			// Sleep for a bit after getting disconnected or failing to connect.
			// If the issue persists, we don't want to waste all our time constantly trying to reconnect.
			task::sleep(Duration::from_secs(1)).await;
		}
	});

	Ok(video_track)
}
