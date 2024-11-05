use std::{env, process::Command};

struct BwrapArgs {
    /// Unshare every namespace supported by default
    unshare_all: bool,
    /// Retain the network namespace (can only combine with unshare_all)
    share_net: bool,
    /// Unset all environment variables
    clear_env: bool,
    new_session: bool,
    die_with_parent: bool,
    /// Custom hostname in the sandbox (requires --unshare-uts)
    hostname: Option<Box<str>>,
    /// Mount new procfs
    proc: Option<Box<str>>,
    /// Mount new dev
    dev: Option<Box<str>>,
    /// Mount new tmpfs
    tmp_fs: Option<Box<str>>,
    /// Set environment variables
    set_env: Vec<(Box<str>, Box<str>)>,
    /// Unset environment variables
    unset_env: Vec<Box<str>>,
    binds: Vec<Bind>,
    dirs: Vec<Dir>,
    symlinks: Vec<(Box<str>, Box<str>)>,
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
}
impl BwrapArgs {
    fn default() -> Self {
        let xdg_runtime_dir = env::var("XDG_RUNTIME_DIR")
            .expect("Environment Variable \"XDG_RUNTIME_DIR\" should exist");

        let home_dir = env::var("HOME").expect("Environment Variable \"HOME\" should exist");
        Self {
            unshare_all: true,
            share_net: false,
            hostname: Some("jail".into()),
            clear_env: true,
            set_env: vec![("PATH".into(), path.into())],
            unset_env: Vec::new(),
            new_session: true,
            die_with_parent: true,
            proc: Some("/proc".into()),
            dev: Some("/dev".into()),
            tmp_fs: Some("/tmp".into()),
            dirs: vec![
                // Basic Directories
                Dir::new("/var".into()),
                Dir::new("/run".into()),
                Dir::with_perms(xdg_runtime_dir.into(), "0700".into()),
            ],
            symlinks: vec![
                ("/run".into(), "/var/run".into()),
                // Merged-usr symlinks
                ("/usr/lib".into(), "/lib".into()),
                ("/usr/lib64".into(), "/lib64".into()),
                ("/usr/bin".into(), "/bin".into()),
                ("/usr/sbin".into(), "/sbin".into()),
            ],
            binds: vec![
                Bind::new("/usr".into()),
                Bind::new("/sys".into()),
                Bind::new("/etc".into()),
                Bind::new(format!("{home_dir}/.cargo/bin").into()),
            ],
        }
    }
}

#[derive(Clone)]
struct Bind {
    bind_type: BindType,
    source: Box<str>,
    // Defaults to source when unset
    destination: Option<Box<str>>,
    ignore_missing_src: bool,
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
enum BindType {
    #[default]
    ReadOnly,
    ReadWrite,
    Dev,
}

/// Create an emtpy directory at path
#[derive(Clone)]
struct Dir {
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
    let program: Vec<_> = env::args().skip(1).collect();

    let bwrap_args = BwrapArgs::default();

    let mut command = bwrap_args.command();

    command.arg("--").args(program);

    dbg!(&command);

    command.spawn().unwrap().wait_with_output().unwrap();
}
