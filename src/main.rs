use clap::{AppSettings, App, Arg};
use failure::format_err;
use futures::StreamExt;
use retina::client::PacketItem;
use rocket::{Request, Response};
use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::Header;
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
	let mut app = App::new("rtsp-to-webrtc")
		.version("0.1.0")
		.author("Alicrow")
		.about("Forwards an RTSP stream as a WebRTC stream.")
		.setting(AppSettings::DeriveDisplayOrder)
		.setting(AppSettings::SubcommandsNegateReqs)
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

	if matches.is_present("FULLHELP") {
		app.print_long_help().unwrap();
		std::process::exit(0);
	}

	let debug = matches.is_present("debug");
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

	let source_stream_settings = common::get_src_stream_settings();

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

	let session_options = retina::client::SessionOptions::default().creds(Some(retina::client::Credentials {username: source_stream_settings.username, password: source_stream_settings.password}) );
	let mut session = retina::client::Session::describe(source_stream_settings.source_url, session_options).await?;
	let video_i = session
		.streams()
		.iter()
		.position(|s| s.media() == "video" && s.encoding_name() == "h264")
		.ok_or_else(|| format_err!("couldn't find H.264 video stream")).unwrap();
	session.setup(video_i, retina::client::SetupOptions::default()).await?;
	let mut session = session.play(retina::client::PlayOptions::default()).await?;

	// Read RTP packets forever and send them to the WebRTC Client
	let video_track_clone = video_track.clone();
	tokio::spawn(async move {
		loop {
			match Pin::new(&mut session).next().await {
				None => {
					println!("stream closed before first frame");
				}
				Some(Err(e)) => {
					println!("encountered error {}", e);
				}
				Some(Ok(PacketItem::Rtp(packet))) => {
					let raw_rtp = packet.raw();
					if let Err(err) = video_track_clone.write(&raw_rtp).await {
						if webrtc::Error::ErrClosedPipe == err {
							// The peerConnection has been closed.
						} else {
							println!("video_track write err: {}", err);
						}
					}
				}
				Some(Ok(_)) => {}
			}
		}
	});

	let _rocket = rocket::build()
		.attach(rest_api::stage(video_track))
		.attach(CORS)
		.launch()
		.await?;

	anyhow::Ok(())
}
