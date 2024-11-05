use std::env;

use crate::{Bind, BindType, BwrapArgs, Dir};

impl Default for BwrapArgs {
    fn default() -> Self {
        let xdg_runtime_dir = env::var("XDG_RUNTIME_DIR")
            .expect("Environment Variable \"XDG_RUNTIME_DIR\" should be set");

        let home_dir = env::var("HOME").expect("Environment Variable \"HOME\" should be set");

        let path = env::var("PATH").expect("Environment Variable \"PATH\" should be set");

        let mut args = Self {
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
        };
        if let Ok(term) = env::var("TERM") {
            args.set_env.push(("TERM".into(), term.into()));
        }

        args
    }
}

impl BwrapArgs {
    /// Tries to interpret all args of the passed command as binds
    pub fn passed_files(input: Vec<String>) -> Self {
        let mut default = Self::default();

        for file in input.into_iter().skip(1) {
            default.binds.push(Bind {
                bind_type: BindType::default(),
                source: file.into(),
                destination: None,
                ignore_missing_src: true,
            });
        }

        default
    }
}
