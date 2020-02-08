#![warn(missing_docs)]
#![warn(unused_extern_crates)]

mod genesis;
mod service;
mod cli;

pub use substrate_cli::{VersionInfo, IntoExit, error};

fn run() -> cli::error::Result<()> {
	let version = VersionInfo {
		name: "gmpc Node",
		commit: env!("VERGEN_SHA_SHORT"),
		version: env!("CARGO_PKG_VERSION"),
		executable_name: "gmpc",
		author: "gmpc-authors",
		description: "gmpc",
		support_url: "support.anonymous.an",
	};
	cli::run(::std::env::args(), cli::Exit, version)
}

error_chain::quick_main!(run);
