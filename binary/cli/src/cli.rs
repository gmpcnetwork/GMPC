extern crate clap;

use crate::service;
use futures::{future, Future, sync::oneshot};
use std::cell::RefCell;
use tokio::runtime::Runtime;
pub use substrate_cli::{VersionInfo, IntoExit, error};
use substrate_cli::{informant, parse_and_execute, NoCustom};
use substrate_service::{ServiceFactory, Roles as ServiceRoles};
use crate::chain_spec;
use std::ops::Deref;


use log::info;
use clap::{Arg, App, SubCommand}

/// Parse command line arguments into service configuration.
pub fn run<I>(args: I, version: String) -> error::Result<()> where
	I: IntoIterator<Item = T>,
{
    

}
