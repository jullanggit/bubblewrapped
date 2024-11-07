use argh::FromArgs;

#[derive(FromArgs)]
/// My little bubblewrap-wrapper
pub struct Cli {
    #[argh(subcommand)]
    pub command: Commands,
}

// TODO: Add commands for calling the different configs
#[derive(FromArgs)]
#[argh(subcommand)]
pub enum Commands {
    Default(Default),
    PassFiles(PassFiles),
    Ls(Ls),
    Nvim(Nvim),
}

#[derive(FromArgs)]
#[argh(subcommand, name = "default")]
/// The default config
pub struct Default {
    #[argh(positional, greedy)]
    /// the command to run in the sandbox
    pub command: Vec<String>,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "pass-files")]
/// Pass files in the command into the Sandbox
pub struct PassFiles {
    #[argh(positional, greedy)]
    /// the command to run in the sandbox
    pub command: Vec<String>,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "ls")]
/// ls
pub struct Ls {
    #[argh(positional, greedy)]
    /// the directories to list (flags will be passed to eza)
    pub dirs: Vec<String>,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "nvim")]
/// nvim
pub struct Nvim {
    #[argh(positional, greedy)]
    /// any additional args to be passed to nvim
    pub args: Vec<String>,
}
