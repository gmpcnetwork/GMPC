#![warn(missing_docs)]

use std::ffi::OsString;
use std::fs::{remove_file, metadata, File, create_dir_all};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::{process, env};

use ansi_term::Colour;
use ctrlc::CtrlC;
use dir::default_hypervisor_path;
use fdlimit::raise_fd_limit;
use ethcore_logger::setup_log;
use parity_ethereum::{start, ExecutionAction};
use parity_daemonize::AsHandle;
use parking_lot::{Condvar, Mutex};

const PLEASE_RESTART_EXIT_CODE: i32 = 69;
const PARITY_EXECUTABLE_NAME: &str = "parity";

#[derive(Debug)]
enum Error {
	BinaryNotFound,
	ExitCode(i32),
	Restart,
	Unknown,
}

fn update_path(name: &str) -> PathBuf {
	let mut dest = default_hypervisor_path();
	dest.push(name);
	dest
}

fn latest_exe_path() -> Result<PathBuf, Error> {
	File::open(update_path("latest")).and_then(|mut f| {
		let mut exe_path = String::new();
		trace!(target: "updater", "latest binary path: {:?}", f);
		f.read_to_string(&mut exe_path).map(|_| update_path(&exe_path))
	})
		.or(Err(Error::BinaryNotFound))
}

fn latest_binary_is_newer(current_binary: &Option<PathBuf>, latest_binary: &Option<PathBuf>) -> bool {
	match (
		current_binary
			.as_ref()
			.and_then(|p| metadata(p.as_path()).ok())
			.and_then(|m| m.modified().ok()),
		latest_binary
			.as_ref()
			.and_then(|p| metadata(p.as_path()).ok())
			.and_then(|m| m.modified().ok())
	) {
		(Some(latest_exe_time), Some(this_exe_time)) if latest_exe_time > this_exe_time => true,
		_ => false,
	}
}

fn set_spec_name_override(spec_name: &str) {
	if let Err(e) = create_dir_all(default_hypervisor_path())
		.and_then(|_| File::create(update_path("spec_name_override"))
			.and_then(|mut f| f.write_all(spec_name.as_bytes())))
	{
		warn!("Couldn't override chain spec: {} at {:?}", e, update_path("spec_name_override"));
	}
}

fn take_spec_name_override() -> Option<String> {
	let p = update_path("spec_name_override");
	let r = File::open(p.clone())
		.ok()
		.and_then(|mut f| {
			let mut spec_name = String::new();
			f.read_to_string(&mut spec_name).ok().map(|_| spec_name)
		});
	let _ = remove_file(p);
	r
}

#[cfg(windows)]
fn global_cleanup() {
	// We need to cleanup all sockets before spawning another Parity process. This makes sure everything is cleaned up.
	// The loop is required because of internal reference counter for winsock dll. We don't know how many crates we use do
	// initialize it. There's at least 2 now.
	for _ in 0..10 {
		unsafe { ::winapi::um::winsock2::WSACleanup(); }
	}
}

#[cfg(not(windows))]
fn global_init() {}

#[cfg(windows)]
fn global_init() {
	// When restarting in the same process this reinits windows sockets.
	unsafe {
		const WS_VERSION: u16 = 0x202;
		let mut wsdata: ::winapi::um::winsock2::WSADATA = ::std::mem::zeroed();
		::winapi::um::winsock2::WSAStartup(WS_VERSION, &mut wsdata);
	}
}

#[cfg(not(windows))]
fn global_cleanup() {}


#[derive(Debug)]
/// Status used to exit or restart the program.
struct ExitStatus {
	/// Whether the program panicked.
	panicking: bool,
	/// Whether the program should exit.
	should_exit: bool,
	/// Whether the program should restart.
	should_restart: bool,
	/// If a restart happens, whether a new chain spec should be used.
	spec_name_override: Option<String>,
}

// Run `locally installed version` of parity (i.e, not installed via `parity-updater`)
// Returns the exit error code.
fn main_direct(force_can_restart: bool) -> i32 {
	global_init();

	let mut conf = {
		let args = std::env::args().collect::<Vec<_>>();
		parity_ethereum::Configuration::parse_cli(&args).unwrap_or_else(|e| e.exit())
	};

	let logger = setup_log(&conf.logger_config()).unwrap_or_else(|e| {
		eprintln!("{}", e);
		process::exit(2)
	});

	if let Some(spec_override) = take_spec_name_override() {
		conf.args.flag_testnet = false;
		conf.args.arg_chain = spec_override;
	}

	// FIXME: `pid_file` shouldn't need to cloned here
	// see: `https://github.com/paritytech/parity-daemonize/pull/13` for more info
	let handle = if let Some(pid) = conf.args.arg_daemon_pid_file.clone() {
		info!("{}", Colour::Blue.paint("starting in daemon mode").to_string());
		let _ = std::io::stdout().flush();

		match parity_daemonize::daemonize(pid) {
			Ok(h) => Some(h),
			Err(e) => {
				error!(
					"{}",
					Colour::Red.paint(format!("{}", e))
				);
				return 1;
			}
		}
	} else {
		None
	};

	let can_restart = force_can_restart || conf.args.flag_can_restart;

	// increase max number of open files
	raise_fd_limit();

	let exit = Arc::new((Mutex::new(ExitStatus {
		panicking: false,
		should_exit: false,
		should_restart: false,
		spec_name_override: None,
	}), Condvar::new()));

	// Double panic can happen. So when we lock `ExitStatus` after the main thread is notified, it cannot be locked
	// again.
	let exiting = Arc::new(AtomicBool::new(false));

	let exec = if can_restart {
		start(
			conf,
			logger,
			{
				let e = exit.clone();
				let exiting = exiting.clone();
				move |new_chain: String| {
					if !exiting.swap(true, Ordering::SeqCst) {
						*e.0.lock() = ExitStatus {
							panicking: false,
							should_exit: true,
							should_restart: true,
							spec_name_override: Some(new_chain),
						};
						e.1.notify_all();
					}
				}
			},
			{
				let e = exit.clone();
				let exiting = exiting.clone();
				move || {
					if !exiting.swap(true, Ordering::SeqCst) {
						*e.0.lock() = ExitStatus {
							panicking: false,
							should_exit: true,
							should_restart: true,
							spec_name_override: None,
						};
						e.1.notify_all();
					}
				}
			},
		)
	} else {
		trace!(target: "mode", "Not hypervised: not setting exit handlers.");
		start(conf, logger, move |_| {}, move || {})
	};

	let res = match exec {
		Ok(result) => match result {
			ExecutionAction::Instant(Some(s)) => {
				println!("{}", s);
				0
			}
			ExecutionAction::Instant(None) => 0,
			ExecutionAction::Running(client) => {
				panic_hook::set_with({
					let e = exit.clone();
					let exiting = exiting.clone();
					move |panic_msg| {
						warn!("Panic occured, see stderr for details");
						eprintln!("{}", panic_msg);
						if !exiting.swap(true, Ordering::SeqCst) {
							*e.0.lock() = ExitStatus {
								panicking: true,
								should_exit: true,
								should_restart: false,
								spec_name_override: None,
							};
							e.1.notify_all();
						}
					}
				});

				CtrlC::set_handler({
					let e = exit.clone();
					let exiting = exiting.clone();
					move || {
						if !exiting.swap(true, Ordering::SeqCst) {
							*e.0.lock() = ExitStatus {
								panicking: false,
								should_exit: true,
								should_restart: false,
								spec_name_override: None,
							};
							e.1.notify_all();
						}
					}
				});

				// so the client has started successfully
				// if this is a daemon, detach from the parent process
				if let Some(mut handle) = handle {
					handle.detach()
				}

				// Wait for signal
				let mut lock = exit.0.lock();
				if !lock.should_exit {
					let _ = exit.1.wait(&mut lock);
				}

				client.shutdown();

				if lock.should_restart {
					if let Some(ref spec_name) = lock.spec_name_override {
						set_spec_name_override(&spec_name.clone());
					}
					PLEASE_RESTART_EXIT_CODE
				} else {
					if lock.panicking {
						1
					} else {
						0
					}
				}
			}
		},
		Err(err) => {
			// error occured during start up
			// if this is a daemon, detach from the parent process
			if let Some(mut handle) = handle {
				handle.detach_with_msg(format!("{}", Colour::Red.paint(&err)))
			}
			eprintln!("{}", err);
			1
		}
	};

	global_cleanup();
	res
}

fn println_trace_main(s: String) {
	if env::var("RUST_LOG").ok().and_then(|s| s.find("main=trace")).is_some() {
		println!("{}", s);
	}
}

macro_rules! trace_main {
	($arg:expr) => (println_trace_main($arg.into()));
	($($arg:tt)*) => (println_trace_main(format!("{}", format_args!($($arg)*))));
}

fn main() {
	trace_main!("Running direct");
	// Otherwise, we're presumably running the version we want. Just run and fall-through.
	process::exit(main_direct(false));
}
