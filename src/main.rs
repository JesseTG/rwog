extern crate caps;
#[macro_use]
extern crate clap;
extern crate groups;
extern crate nix;
extern crate passwd;

use caps::{CapSet, Capability};
use clap::{App, AppSettings, Arg};
use groups::get_group_by_name;
use nix::Error;
use nix::errno::Errno;
use nix::libc::{gid_t, uid_t};
use nix::unistd::*;
use passwd::Passwd;
use std::cmp::Ordering;
use std::os::unix::process::CommandExt;
use std::process::{exit, Command};

const GROUPS_ARG: &'static str = "groups";
const COMMANDS_ARG: &'static str = "command-with-args";

fn main() {
    let passwd = Passwd::from_uid(uid_t::from(getuid()))
        .expect("Couldn't get /etc/passwd entry for active user (your system might be broken)");

    let app: App = app_from_crate!();
    let args = app
        .setting(AppSettings::ArgRequiredElseHelp)
        .arg(
            Arg::with_name(GROUPS_ARG)
                .alias("group")
                .short("g")
                .required(true)
                .help("Groups to drop (at least 1)")
                .value_delimiter(",")
                .multiple(true)
                .empty_values(false)
                .long_help(include_str!("help/groups-long.txt"))
                .min_values(1) // TODO: Can I put possible_values in here usefully?
        )
        .arg(Arg::with_name(COMMANDS_ARG)
            .aliases(&["command", "commands", "cmd", "cmds", "prog", "progs", "programs", "programs"])
            .multiple(true)
            .last(true)
            .default_value(&passwd.shell)
            .empty_values(false)
            .help("Command and arguments to run")
            .long_help(include_str!("help/commands-long.txt"))
        )
        .about(include_str!("help/about.txt"))
        .get_matches();

    let mut groups_to_drop: Vec<Gid> = args.values_of(GROUPS_ARG)
        .expect("The arg parser should've handled this already, file a bug")
        .filter_map(|name| match get_group_by_name(name) {
            Some(group) => Some(Gid::from_raw(group.gid)),
            None => {
                eprintln!("{} is not a valid group, ignoring", name);
                None
            }
        })
        .collect();

    // let mut groups_to_drop = groups_to_drop
    groups_to_drop.sort_by(|a, b| {
        if a == b {
            Ordering::Equal
        } else {
            Ordering::Less
        }
    });
    groups_to_drop.dedup();
    // dedup only removes duplicate items that are next to each other
    // I don't care if the elements are actually sorted or not

    if groups_to_drop.is_empty() {
        eprintln!("No valid groups listed");
        eprintln!("{}", args.usage());
        exit(1); // TODO: Should I just allow this trivial behavior?  Yes.  How?
    }

    let command: Vec<_> = args.values_of(COMMANDS_ARG)
        .expect("The arg parser should've handled this command already, file a bug")
        .collect();

    let groups: Vec<Gid> = match getgroups() {
        Ok(mut groups) => {
            groups.retain(|g| !groups_to_drop.contains(g));
            groups
        }
        Err(e) => {
            eprintln!("{}", e);
            exit(1);
        }
    };

    eprintln!("Dropped groups: {:?}", groups_to_drop);
    eprintln!("Remaining groups: {:?}", groups);
    eprintln!("Command to run: {:?}", command);

    match setgroups(&groups[..]) {
        Err(Error::Sys(Errno::EPERM)) => {
            eprintln!("Insufficient permissions to reduce groups.");
            eprintln!("Please run 'setcap $(which rwog) cap_setgid=pe' as root");
            exit(1);
        }
        Err(e) => {
            eprintln!("Failed to reduce groups: {}", e);
            exit(1);
        }
        _ => {} // OK: Nothing happened
    };

    if let Err(e) = caps::drop(None, CapSet::Effective, Capability::CAP_SETGID) {
        eprintln!("Failed to drop capabilities: {}", e);
        exit(1);
    }

    let error = Command::new(&command[0]).args(&command[1..]).exec();
    // Should not reach this point if the command succeeds
    eprintln!("Failed to execute {:?}: {}", command, error);

    exit(255i32);
}
