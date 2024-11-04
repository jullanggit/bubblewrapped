use std::{env, process::Command};

struct BwrapArgs<'a> {
    /// Unshare every namespace supported by default
    unshare_all: bool,
    /// Retain the network namespace (can only combine with unshare_all)
    share_net: bool,
    /// Unset all environment variables
    clear_env: bool,
    new_session: bool,
    die_with_parent: bool,
    /// Custom hostname in the sandbox (requires --unshare-uts)
    hostname: Option<&'a str>,
    /// Mount new procfs
    proc: Option<&'a str>,
    /// Mount new dev
    dev: Option<&'a str>,
    /// Mount new tmpfs
    tmp_fs: Option<&'a str>,
    /// Set environment variables
    set_env: Vec<(&'a str, &'a str)>,
    /// Unset environment variables
    unset_env: Vec<&'a str>,
    binds: Vec<Bind<'a>>,
    dirs: Vec<Dir<'a>>,
    symlinks: Vec<(&'a str, &'a str)>,
}
impl BwrapArgs<'_> {
    fn args(&self) -> Vec<&str> {
        let mut args = Vec::new();

        if self.unshare_all {
            args.push("--unshare-all");

            if self.share_net {
                args.push("--share-net");
            }
        }

        if self.clear_env {
            args.push("--clearenv");
        }

        if self.new_session {
            args.push("--new-session");
        }

        if self.die_with_parent {
            args.push("--die-with-parent");
        }

        // TODO: Add validation if this argument is allowed (unshare-uts)
        if let Some(hostname) = self.hostname {
            args.push("--hostname");
            args.push(hostname);
        }

        if let Some(proc) = self.proc {
            args.push("--proc");
            args.push(proc);
        }
        if let Some(dev) = self.dev {
            args.push("--dev");
            args.push(dev);
        }
        if let Some(tmp_fs) = self.tmp_fs {
            args.push("--tmpfs");
            args.push(tmp_fs);
        }

        for (var, value) in &self.set_env {
            args.push("--setenv");
            args.push(var);
            args.push(value);
        }

        for var in &self.unset_env {
            args.push("--unsetenv");
            args.push(var);
        }

        for bind in &self.binds {
            args.push(match (bind.bind_type, bind.ignore_missing_src) {
                (BindType::ReadOnly, false) => "--ro-bind",
                (BindType::ReadOnly, true) => "--ro-bind-try",
                (BindType::ReadWrite, false) => "--bind",
                (BindType::ReadWrite, true) => "--bind-try",
                (BindType::Dev, false) => "--dev-bind",
                (BindType::Dev, true) => "--dev-bind-try",
            });
            args.push(bind.source);
            args.push(bind.destination);
        }

        for dir in &self.dirs {
            if let Some(permissions) = dir.permissions {
                args.push("--perms");
                args.push(permissions);
            }
            args.push("--dir");
            args.push(dir.path);
        }

        for (source, destination) in &self.symlinks {
            args.push("--symlink");
            args.push(source);
            args.push(destination);
        }

        args
    }
    fn command(&self) -> Command {
        let args = self.args();
        let mut command = Command::new("bwrap");
        command.args(args);
        command
    }
}
impl<'a> BwrapArgs<'a> {
    // TODO: Somehow make the xdg_runtime_dir a const, so you dont have to pass it in
    fn new(xdg_runtime_dir: &'a str) -> Self {
        Self {
            unshare_all: true,
            share_net: false,
            hostname: Some("jail"),
            clear_env: true,
            set_env: Vec::new(),
            unset_env: Vec::new(),
            new_session: true,
            die_with_parent: true,
            proc: Some("/proc"),
            dev: Some("/dev"),
            tmp_fs: Some("/tmp"),
            dirs: vec![
                // Basic Directories
                Dir::new("/var"),
                Dir::new("/run"),
                Dir::with_perms(xdg_runtime_dir, "0700"),
            ],
            symlinks: vec![
                ("../run", "/var/run"),
                // Merged-usr symlinks
                ("/usr/lib", "/lib"),
                ("/usr/lib64", "/lib64"),
                ("/usr/bin", "/bin"),
                ("/usr/sbin", "/sbin"),
            ],
            binds: vec![
                Bind::new(BindType::ReadOnly, "/usr"),
                Bind::new(BindType::ReadOnly, "/sys"),
                Bind::new(BindType::ReadOnly, "/etc"),
                Bind::new(BindType::ReadOnly, "~/.cargo/bin"),
            ],
        }
    }
}

struct Bind<'a> {
    bind_type: BindType,
    source: &'a str,
    // TODO: See what this should be set to
    destination: &'a str,
    ignore_missing_src: bool,
}

impl<'a> Bind<'a> {
    fn new(bind_type: BindType, source: &'a str) -> Self {
        Self {
            bind_type,
            source,
            destination: source,
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
struct Dir<'a> {
    // Really a 9-bit flag
    permissions: Option<&'a str>,
    path: &'a str,
}
impl<'a> Dir<'a> {
    fn new(path: &'a str) -> Self {
        Self {
            permissions: None,
            path,
        }
    }
    fn with_perms(path: &'a str, permissions: &'a str) -> Self {
        Self {
            permissions: Some(permissions),
            path,
        }
    }
}

fn main() {
    let program: Vec<_> = env::args().skip(2).collect();

    let xdg_runtime_dir =
        env::var("XDG_RUNTIME_DIR").expect("Environment Variable XDG_RUNTIME_DIR should exist");

    let bwrap_args = BwrapArgs::new(&xdg_runtime_dir);

    let mut command = bwrap_args.command();
    dbg!(&command);
    command
        .args(program)
        .spawn()
        .unwrap()
        .wait_with_output()
        .unwrap();
}
