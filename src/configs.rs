use std::env;

use crate::{Bind, BindType, BwrapArgs, Dir};

impl Default for BwrapArgs {
    fn default() -> Self {
        let xdg_runtime_dir = xdg_runtime_dir();

        let home_dir = home_dir();

        let path = env::var("PATH").unwrap();

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

fn home_dir() -> String {
    env::var("HOME").unwrap()
}

fn xdg_runtime_dir() -> String {
    env::var("XDG_RUNTIME_DIR").unwrap()
}

impl BwrapArgs {
    pub fn pass_files(mut self, input: Vec<String>, skip_first: bool) -> Self {
        for file in input.into_iter().skip(skip_first as usize) {
            self.binds.push(Bind::new(file.into()));
        }
        self
    }
    pub fn ls(input: &mut Vec<String>) -> Self {
        input.insert(0, "eza".into());

        let paths: Vec<_> = input
            .iter()
            .skip(1) // Skip the "eza"
            .filter(|part| !part.starts_with('-'))
            .cloned()
            .collect();

        if paths.is_empty() {
            input.push(working_directory())
        }

        Self::default().pass_files(paths, false)
    }

    fn cur_dir_rw(mut self) -> Self {
        self.binds.push(Bind::with_bind_type(
            working_directory().into(),
            BindType::ReadWrite,
        ));
        self
    }

    fn wl_socket(mut self) -> Self {
        let wayland_display = env::var("WAYLAND_DISPLAY").unwrap().into_boxed_str();
        let xdg_runtime_dir = xdg_runtime_dir();

        self.set_env
            .push(("WAYLAND_DISPLAY".into(), wayland_display.clone()));

        self.binds.push(Bind::with_bind_type(
            format!("{xdg_runtime_dir}/{wayland_display}").into(),
            BindType::ReadWrite,
        ));

        self
    }

    pub fn nvim(nvim_args: &mut Vec<String>) -> Self {
        nvim_args.insert(0, "nvim".into());

        let home_dir = home_dir();

        let additional_paths = [
            Bind::new(format!("{home_dir}/.config/nvim").into()),
            Bind::with_bind_type(
                format!("{home_dir}/.cache/nvim").into(),
                BindType::ReadWrite,
            ),
            Bind::with_bind_type(
                format!("{home_dir}/.local/share/nvim").into(),
                BindType::ReadWrite,
            ),
            Bind::with_bind_type(
                format!("{home_dir}/.local/state/nvim").into(),
                BindType::ReadWrite,
            ),
        ];

        let mut args = Self::default().cur_dir_rw().wl_socket();

        args.binds.extend(additional_paths);

        args
    }
}

fn working_directory() -> String {
    env::current_dir().unwrap().to_str().unwrap().into()
}
