extern crate caps;
#[macro_use]
extern crate clap;
extern crate nix;
extern crate users;

use users::*;
use users::os::unix::*;
use caps::{CapSet, Capability};
use clap::{App, AppSettings, Arg};
use nix::Error;
use nix::errno::Errno;
use nix::unistd::*;
use nix::unistd::Gid;
use std::convert::Into;
use std::os::unix::process::CommandExt;
use std::process::{exit, Command};

const GROUPS_ARG: &'static str = "groups";
const COMMANDS_ARG: &'static str = "command-with-args";

fn main() {
    let user = get_current_uid();
    let user = match get_user_by_uid(user) {
        Some(user) => user,
        None => {
            eprintln!("Failed to get user info for uid {}", user);
            exit(1);
        }
    };
    let user_shell = user.shell();
    let user_shell = match user_shell.to_str() {
        Some(s) => s,
        None => {
            eprintln!("Failed to convert user shell {:?} to str", user_shell);
            exit(1);
        }
    };
    let groups: Vec<Group> = getgroups()
        .expect("Failed to get supplementary group list")
        .iter()
        .filter_map(|g| get_group_by_gid((*g).into()))
        .collect();

    let mut group_names: Vec<&str> = groups.iter().map(|g| g.name()).collect();
    let group = get_current_gid();
    let group = match get_group_by_gid(group) {
        Some(group) => group,
        None => {
            eprintln!("Failed to get user info for gid {}", group);
            exit(1);
        }
    };
    group_names.retain(|n| n != &group.name());

    let app: App = app_from_crate!();
    let args = app.setting(AppSettings::ArgRequiredElseHelp)
        .arg(
            Arg::with_name(GROUPS_ARG)
                .alias("group")
                .short("g")
                .long(GROUPS_ARG)
                .required(true)
                .help("Groups to drop (at least 1)")
                .value_delimiter(",")
                .multiple(true)
                .empty_values(false)
                .long_help(include_str!("help/groups-long.txt"))
                .hide_possible_values(true)
                .possible_values(&group_names)
                .min_values(1),
        )
        .arg(
            Arg::with_name(COMMANDS_ARG)
                .aliases(&[
                    "command", "commands", "cmd", "cmds", "prog", "progs", "programs", "programs"
                ])
                .multiple(true)
                .last(true)
                .default_value(user_shell)
                .empty_values(false)
                .help("Command and arguments to run")
                .long_help(include_str!("help/commands-long.txt")),
        )
        .about(include_str!("help/about.txt"))
        .get_matches();

    let groups_to_drop: Vec<Group> = args.values_of(GROUPS_ARG)
        .expect("The arg parser should've handled this already, file a bug")
        .filter_map(get_group_by_name)
        .collect();

    let mut gids_to_drop: Vec<gid_t> = groups_to_drop.iter().map(|g| g.gid()).collect();
    gids_to_drop.sort();
    gids_to_drop.dedup();

    let remaining_groups: Vec<Gid> = groups
        .iter()
        .filter(|g| !gids_to_drop.contains(&g.gid()))
        .map(|g| Gid::from_raw(g.gid()))
        .collect();

    let command: Vec<_> = args.values_of(COMMANDS_ARG)
        .expect("The arg parser should've handled this command already, file a bug")
        .collect();

    eprintln!("Dropped groups: {:?}", groups_to_drop);
    eprintln!("Remaining groups: {:?}", groups);
    eprintln!("Command to run: {:?}", command);

    match setgroups(&remaining_groups[..]) {
        Err(Error::Sys(Errno::EPERM)) => {
            eprintln!(include_str!("error/permission-denied.txt"));
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
