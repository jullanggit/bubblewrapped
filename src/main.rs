use anyhow::{anyhow, Result};
use std::{
    path::{Path, PathBuf},
    process::{exit, Command},
};

use cli::{Cli, Commands};

mod cli;
mod configs;

#[derive(Debug)]
pub struct BwrapArgs {
    /// Unshare every namespace supported by default
    pub unshare_all: bool,
    /// Retain the network namespace (can only combine with unshare_all)
    pub share_net: bool,
    /// Unset all environment variables
    pub clear_env: bool,
    pub new_session: bool,
    pub die_with_parent: bool,
    pub follow_symlinks: bool,
    /// Custom hostname in the sandbox (requires --unshare-uts)
    pub hostname: Option<Box<str>>,
    /// Mount new procfs
    pub proc: Option<PathBox>,
    /// Mount new dev
    pub dev: Option<PathBox>,
    /// Mount new tmpfs
    pub tmp_fs: Option<PathBox>,
    /// Set environment variables
    // TODO: Maybe change this to a hashmap
    pub set_env: Vec<(Box<str>, Box<str>)>,
    /// Unset environment variables
    pub unset_env: Vec<Box<str>>,
    pub binds: Vec<Bind>,
    pub dirs: Vec<Dir>,
    pub symlinks: Vec<(PathBox, PathBox)>,
}
impl BwrapArgs {
    // TODO: Maybe add assertions that paths are absolute
    fn args(&self) -> Vec<Box<str>> {
        let mut args = Vec::new();

        if self.unshare_all {
            args.push("--unshare-all".into());

            if self.share_net {
                args.push("--share-net".into());
            }
        } else if self.share_net {
            eprintln!("share-net can only be combined with unshare-all");
            exit(1);
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
            args.push(proc.into());
        }
        if let Some(dev) = self.dev.clone() {
            args.push("--dev".into());
            args.push(dev.into());
        }
        if let Some(tmp_fs) = self.tmp_fs.clone() {
            args.push("--tmpfs".into());
            args.push(tmp_fs.into());
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
            args.push(match (bind.bind_type, bind.allow_missing_src) {
                (BindType::ReadOnly, false) => "--ro-bind".into(),
                (BindType::ReadOnly, true) => "--ro-bind-try".into(),
                (BindType::ReadWrite, false) => "--bind".into(),
                (BindType::ReadWrite, true) => "--bind-try".into(),
                (BindType::Dev, false) => "--dev-bind".into(),
                (BindType::Dev, true) => "--dev-bind-try".into(),
            });

            let source: Box<str> = bind.source.into();
            args.push(source.clone());
            args.push(
                bind.destination
                    .map_or(source, |destination| destination.into()),
            );
        }

        for dir in self.dirs.iter().cloned() {
            if let Some(permissions) = dir.permissions {
                args.push("--perms".into());
                args.push(permissions);
            }
            args.push("--dir".into());
            args.push(dir.path.into());
        }

        for (source, destination) in self.symlinks.iter().cloned() {
            args.push("--symlink".into());
            args.push(source.into());
            args.push(destination.into());
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

    /// Adds (deduplicated) bind. Errors when changing bind type to/from dev
    fn add_bind(&mut self, bind: Bind) -> Result<()> {
        for existing_bind in &mut self.binds {
            if existing_bind.source == bind.source {
                match (existing_bind.bind_type, bind.bind_type) {
                    (BindType::Dev, BindType::Dev) => {}
                    (BindType::Dev, _) | (_, BindType::Dev) => {
                        Err(anyhow!("Tried to change bind type to/from dev"))?
                    }
                    (BindType::ReadWrite, _) | (_, BindType::ReadWrite) => {
                        existing_bind.bind_type = BindType::ReadWrite
                    }
                    _ => {}
                }
                return Ok(());
            }
        }
        self.binds.push(bind);
        Ok(())
    }

    /// Adds (deduplicated) env variable
    fn add_env(&mut self, (key, value): (Box<str>, Box<str>)) -> Result<()> {
        for (existing_key, existing_value) in &self.set_env {
            if key == *existing_key {
                if value != *existing_value {
                    return Err(anyhow!("Tried to overwrite env variable '{key}' with '{value}' (previous: '{existing_value}' "));
                } else {
                    break;
                }
            }
        }

        self.set_env.push((key, value));

        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct Bind {
    pub bind_type: BindType,
    pub source: PathBox,
    // Defaults to source when unset
    pub destination: Option<PathBox>,
    pub allow_missing_src: bool,
}

impl Bind {
    fn new(source: PathBox) -> Result<Self> {
        Self::_new_inner(source, None, BindType::default(), false)
    }
    fn with_bind_type(source: PathBox, bind_type: BindType) -> Result<Self> {
        Self::_new_inner(source, None, bind_type, false)
    }
    pub fn _new_inner(
        source: PathBox,
        destination: Option<PathBox>,
        bind_type: BindType,
        allow_missing_src: bool,
    ) -> Result<Self> {
        if !allow_missing_src && !source.0.exists() {
            Err(anyhow!(
                "Source for binding doesnt exist: {}",
                source.0.display()
            ))?
        }
        Ok(Self {
            bind_type,
            source,
            destination,
            allow_missing_src,
        })
    }
}

#[derive(Default, Clone, Copy, Debug)]
pub enum BindType {
    #[default]
    ReadOnly,
    ReadWrite,
    Dev,
}

/// Create an empty directory at path
#[derive(Clone, Debug)]
pub struct Dir {
    // Really a 9-bit flag
    permissions: Option<Box<str>>,
    path: PathBox,
}
impl Dir {
    fn new(path: PathBox) -> Self {
        Self {
            permissions: None,
            path,
        }
    }
    fn with_perms(path: PathBox, permissions: Box<str>) -> Self {
        Self {
            permissions: Some(permissions),
            path,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
/// A wrapper type around Box<Path>
pub struct PathBox(pub Box<Path>);
impl From<&str> for PathBox {
    fn from(value: &str) -> Self {
        Self(Path::new(value).into())
    }
}
impl From<String> for PathBox {
    fn from(value: String) -> Self {
        Self(Path::new(&value).into())
    }
}
impl From<PathBuf> for PathBox {
    fn from(value: PathBuf) -> Self {
        Self(value.into())
    }
}
impl From<PathBox> for Box<str> {
    fn from(value: PathBox) -> Self {
        value.0.to_str().expect("Path should be valid utf-8").into()
    }
}

fn main() -> Result<()> {
    let cli_args: Cli = argh::from_env();

    let mut args = BwrapArgs::default()?;

    let command = match cli_args.command {
        Commands::Default(input) => input.command,
        Commands::PassFiles(input) => {
            args.pass_files(&input.command, true)?;
            input.command
        }
        Commands::Ls(mut input) => {
            args.ls(&input.dirs)?;

            input.dirs.insert(0, "eza".into());
            input.dirs
        }
        Commands::Nvim(mut input) => {
            args.nvim()?;

            input.args.insert(0, "nvim".into());
            input.args
        }
    };
    if args.follow_symlinks {
        args.add_symlinks()?;
    }

    if command.is_empty() {
        eprintln!("Please supply a command to run in the sandbox");
        exit(1);
    }

    args.run(command);

    Ok(())
}
