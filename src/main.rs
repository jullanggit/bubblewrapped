use std::process::{exit, Command};

use clap::Parser;
use cli::{Cli, Commands};

mod cli;
mod configs;

pub struct BwrapArgs {
    /// Unshare every namespace supported by default
    pub unshare_all: bool,
    /// Retain the network namespace (can only combine with unshare_all)
    pub share_net: bool,
    /// Unset all environment variables
    pub clear_env: bool,
    pub new_session: bool,
    pub die_with_parent: bool,
    /// Custom hostname in the sandbox (requires --unshare-uts)
    pub hostname: Option<Box<str>>,
    /// Mount new procfs
    pub proc: Option<Box<str>>,
    /// Mount new dev
    pub dev: Option<Box<str>>,
    /// Mount new tmpfs
    pub tmp_fs: Option<Box<str>>,
    /// Set environment variables
    pub set_env: Vec<(Box<str>, Box<str>)>,
    /// Unset environment variables
    pub unset_env: Vec<Box<str>>,
    pub binds: Vec<Bind>,
    pub dirs: Vec<Dir>,
    pub symlinks: Vec<(Box<str>, Box<str>)>,
}
impl BwrapArgs {
    fn args(&self) -> Vec<Box<str>> {
        let mut args = Vec::new();

        if self.unshare_all {
            args.push("--unshare-all".into());

            if self.share_net {
                args.push("--share-net".into());
            }
        }

        if self.clear_env {
            args.push("--clearenv".into());
        }

        if self.new_session {
            args.push("--new-session".into());
        }

        if self.die_with_parent {
            args.push("--die-with-parent".into());
        }

        // TODO: Add validation if this argument is allowed (unshare-uts)
        if let Some(hostname) = self.hostname.clone() {
            args.push("--hostname".into());
            args.push(hostname);
        }

        if let Some(proc) = self.proc.clone() {
            args.push("--proc".into());
            args.push(proc);
        }
        if let Some(dev) = self.dev.clone() {
            args.push("--dev".into());
            args.push(dev);
        }
        if let Some(tmp_fs) = self.tmp_fs.clone() {
            args.push("--tmpfs".into());
            args.push(tmp_fs);
        }

        for (var, value) in self.set_env.iter().cloned() {
            args.push("--setenv".into());
            args.push(var);
            args.push(value);
        }

        for var in self.unset_env.iter().cloned() {
            args.push("--unsetenv".into());
            args.push(var);
        }

        for bind in self.binds.iter().cloned() {
            args.push(match (bind.bind_type, bind.ignore_missing_src) {
                (BindType::ReadOnly, false) => "--ro-bind".into(),
                (BindType::ReadOnly, true) => "--ro-bind-try".into(),
                (BindType::ReadWrite, false) => "--bind".into(),
                (BindType::ReadWrite, true) => "--bind-try".into(),
                (BindType::Dev, false) => "--dev-bind".into(),
                (BindType::Dev, true) => "--dev-bind-try".into(),
            });
            args.push(bind.source.clone());
            args.push(bind.destination.unwrap_or(bind.source));
        }

        for dir in self.dirs.iter().cloned() {
            if let Some(permissions) = dir.permissions {
                args.push("--perms".into());
                args.push(permissions);
            }
            args.push("--dir".into());
            args.push(dir.path);
        }

        for (source, destination) in self.symlinks.iter().cloned() {
            args.push("--symlink".into());
            args.push(source);
            args.push(destination);
        }

        args
    }
    fn command(&self) -> Command {
        let args = self.args();

        let mut command = Command::new("bwrap");

        // .args(args.map(&*)) doesnt work because of reference to variable owned by local function
        for arg in args {
            command.arg(&*arg);
        }
        command
    }

    fn run(&self, input: Vec<String>) {
        let mut command = self.command();

        command.arg("--").args(input);

        command.spawn().unwrap().wait_with_output().unwrap();
    }
}

#[derive(Clone)]
pub struct Bind {
    pub bind_type: BindType,
    pub source: Box<str>,
    // Defaults to source when unset
    pub destination: Option<Box<str>>,
    pub ignore_missing_src: bool,
}

impl Bind {
    fn new(source: Box<str>) -> Self {
        Self::with_bind_type(source, BindType::default())
    }
    fn with_bind_type(source: Box<str>, bind_type: BindType) -> Self {
        Self {
            bind_type,
            source,
            destination: None,
            ignore_missing_src: false,
        }
    }
}

#[derive(Default, Clone, Copy)]
pub enum BindType {
    #[default]
    ReadOnly,
    ReadWrite,
    Dev,
}

/// Create an emtpy directory at path
#[derive(Clone)]
pub struct Dir {
    // Really a 9-bit flag
    permissions: Option<Box<str>>,
    path: Box<str>,
}
impl Dir {
    fn new(path: Box<str>) -> Self {
        Self {
            permissions: None,
            path,
        }
    }
    fn with_perms(path: Box<str>, permissions: Box<str>) -> Self {
        Self {
            permissions: Some(permissions),
            path,
        }
    }
}

fn main() {
    let cli_args = Cli::parse();

    let (bwrap_args, input) = match cli_args.command {
        Commands::Default { input } => (BwrapArgs::default(), input),
        Commands::PassFiles { input } => {
            (BwrapArgs::default().pass_files(input.clone(), true), input)
        }
        Commands::Ls { mut files } => (BwrapArgs::ls(&mut files), files),
        Commands::Nvim { mut args } => (BwrapArgs::nvim(&mut args), args),
    };

    if input.is_empty() {
        eprintln!("Please supply a command to run in the sandbox");
        exit(1);
    }
    bwrap_args.run(input);
}
