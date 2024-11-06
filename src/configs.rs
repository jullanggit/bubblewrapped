use anyhow::{anyhow, Result};
use std::env;

use crate::{Bind, BindType, BwrapArgs, Dir, PathBox};

impl BwrapArgs {
    // Not a trait impl because of the result
    #[expect(clippy::should_implement_trait)]
    pub fn default() -> Result<Self> {
        let xdg_runtime_dir = xdg_runtime_dir()?;

        let home_dir = home_dir()?;

        let path = env::var("PATH")?;

        let mut args = Self {
            unshare_all: true,
            share_net: false,
            clear_env: true,
            new_session: true,
            die_with_parent: true,
            follow_symlinks: true,
            hostname: Some("jail".into()),
            set_env: vec![
                ("PATH".into(), path.into()),
                ("XDG_RUNTIME_DIR".into(), xdg_runtime_dir.clone().into()),
            ],
            unset_env: Vec::new(),
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
                Bind::new("/usr".into())?,
                Bind::new("/sys".into())?,
                Bind::new("/etc".into())?,
                Bind::new(format!("{home_dir}/.cargo/bin").into())?,
            ],
        };
        if let Ok(term) = env::var("TERM") {
            args.set_env.push(("TERM".into(), term.into()));
        }

        Ok(args)
    }
}

fn home_dir() -> Result<String> {
    Ok(env::var("HOME")?)
}

fn xdg_runtime_dir() -> Result<String> {
    Ok(env::var("XDG_RUNTIME_DIR").unwrap())
}

impl BwrapArgs {
    pub fn pass_files(mut self, input: Vec<String>, skip_first: bool) -> Result<Self> {
        for file in input.into_iter().skip(skip_first as usize) {
            self.binds.push(Bind::new(file.into())?);
        }
        Ok(self)
    }
    pub fn ls(input: &mut Vec<String>) -> Result<Self> {
        input.insert(0, "eza".into());

        let mut paths: Vec<_> = input
            .iter()
            .skip(1) // Skip the "eza"
            .filter(|part| !part.starts_with('-'))
            .cloned()
            .collect();

        if paths.is_empty() {
            paths.push(working_directory()?)
        }

        Self::default()?.pass_files(paths, false)
    }

    fn cur_dir_rw(mut self) -> Result<Self> {
        self.binds.push(Bind::with_bind_type(
            working_directory()?.into(),
            BindType::ReadWrite,
        )?);
        Ok(self)
    }

    fn wl_socket(mut self) -> Result<Self> {
        let wayland_display = env::var("WAYLAND_DISPLAY")?.into_boxed_str();
        let xdg_runtime_dir = xdg_runtime_dir()?;

        self.set_env
            .push(("WAYLAND_DISPLAY".into(), wayland_display.clone()));

        self.binds.push(Bind::with_bind_type(
            format!("{xdg_runtime_dir}/{wayland_display}").into(),
            BindType::ReadWrite,
        )?);

        Ok(self)
    }

    pub fn nvim(nvim_args: &mut Vec<String>) -> Result<Self> {
        nvim_args.insert(0, "nvim".into());

        let home_dir = home_dir()?;

        let additional_paths = [
            Bind::new(format!("{home_dir}/.config/nvim").into())?,
            Bind::with_bind_type(
                format!("{home_dir}/.cache/nvim").into(),
                BindType::ReadWrite,
            )?,
            Bind::with_bind_type(
                format!("{home_dir}/.local/share/nvim").into(),
                BindType::ReadWrite,
            )?,
            Bind::with_bind_type(
                format!("{home_dir}/.local/state/nvim").into(),
                BindType::ReadWrite,
            )?,
        ];

        let mut args = Self::default()?.cur_dir_rw()?.wl_socket()?;

        args.binds.extend(additional_paths);

        Ok(args)
    }
    // TODO: Remove .clone()'s
    pub fn add_symlinks(mut self) -> Result<Self> {
        let len = self.binds.len();

        for i in 0..len {
            let bind = self.binds[i].clone();

            if bind.source.0.is_symlink() {
                self.add_symlink_dst(bind.source.clone(), bind.clone())?;
            } else if bind.source.0.is_dir() {
                for result in bind.source.0.read_dir()? {
                    let path = PathBox::from(result?.path());

                    if path.0.is_symlink() {
                        self.add_symlink_dst(path, bind.clone())?;
                    }
                }
            }
        }

        Ok(self)
    }

    fn add_symlink_dst(&mut self, source: PathBox, bind: Bind) -> Result<()> {
        // Where the source symlink points to
        let src_dst = source.0.read_link()?;
        self.binds.push(Bind::_new_inner(
            src_dst.into(),
            bind.destination.clone(),
            bind.bind_type,
            bind.allow_missing_src,
        )?);

        Ok(())
    }
}

fn working_directory() -> Result<String> {
    Ok(env::current_dir()?
        .to_str()
        .ok_or(anyhow!("Path is not valid utf-8"))?
        .into())
}
