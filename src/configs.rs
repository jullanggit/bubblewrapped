use anyhow::{anyhow, Result};
use std::{env, io};

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

        let include_if_exists = ["TERM", "COLORTERM"];
        for key in include_if_exists {
            if let Ok(value) = env::var(key) {
                args.set_env.push((key.into(), value.into()));
            }
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
    pub fn pass_files(&mut self, input: &[String], skip_first: bool) -> Result<()> {
        for file in input.iter().skip(skip_first as usize) {
            if !file.starts_with('-') {
                self.add_bind(Bind::new(PathBox::from(file.clone()))?)?;
            }
        }
        Ok(())
    }
    pub fn ls(&mut self, input: &[String]) -> Result<()> {
        let num_paths = input.iter().filter(|part| !part.starts_with('-')).count();

        let mut paths = input;

        // TODO: Remove the need to compute the working directory if it isnt needed
        let working_directory = &[working_directory()?];
        if num_paths == 0 {
            paths = working_directory;
        }

        // If the root folder is included, dont bother binding or symlinking anything (also avoids
        // an error)
        if paths.contains(&"/".into()) {
            self.binds = vec![Bind::new("/".into())?];
            self.symlinks.clear();

            Ok(())
        } else {
            self.pass_files(paths, false)
        }
    }

    fn cur_dir_rw(&mut self) -> Result<()> {
        self.add_bind(Bind::with_bind_type(
            working_directory()?.into(),
            BindType::ReadWrite,
        )?)
    }

    fn wl_socket(&mut self) -> Result<()> {
        let wayland_display = env::var("WAYLAND_DISPLAY")?.into_boxed_str();
        let xdg_runtime_dir = xdg_runtime_dir()?;

        self.add_env(("WAYLAND_DISPLAY".into(), wayland_display.clone()))?;

        self.add_bind(Bind::with_bind_type(
            format!("{xdg_runtime_dir}/{wayland_display}").into(),
            BindType::ReadWrite,
        )?)
    }

    pub fn nvim(&mut self) -> Result<()> {
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
            // Because of markdown-preview.nvim
            Bind::with_bind_type(
                format!("{home_dir}/.cache/yarn").into(),
                BindType::ReadWrite,
            )?,
        ];

        for bind in additional_paths {
            self.add_bind(bind)?;
        }

        self.cur_dir_rw()?;
        self.wl_socket()
    }

    // TODO: Remove .clone()'s
    pub fn add_symlinks(&mut self) -> Result<()> {
        let len = self.binds.len();

        for i in 0..len {
            let bind = self.binds[i].clone();

            if bind.source.0.is_symlink() {
                self.add_symlink_dst(bind.source.clone(), bind.clone())?;
            } else if bind.source.0.is_dir() {
                for result in bind.source.0.read_dir()? {
                    let path = PathBox::from(result?.path());

                    if path == "/etc/mtab".into() {
                        continue;
                    }

                    if path.0.is_symlink() {
                        self.add_symlink_dst(path, bind.clone())?;
                    }
                }
            }
        }

        Ok(())
    }

    fn add_symlink_dst(&mut self, source: PathBox, bind: Bind) -> Result<()> {
        // Where the source symlink points to
        match source.0.canonicalize() {
            Ok(dst) => {
                self.add_bind(Bind::_new_inner(
                    dst.into(),
                    bind.destination.clone(),
                    bind.bind_type,
                    bind.allow_missing_src,
                )?)?;
            }
            Err(e) => {
                // Allow not found to happen, as we dont want to error on broken symlinks
                if e.kind() != io::ErrorKind::NotFound {
                    Err(e)?
                }
            }
        }

        Ok(())
    }
}

fn working_directory() -> Result<String> {
    Ok(env::current_dir()?
        .to_str()
        .ok_or(anyhow!("Path is not valid utf-8"))?
        .into())
}
