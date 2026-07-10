use argh::FromArgs;

/// molt — transform this `fuz_template` clone into your own project, then
/// molt deletes itself. Run with no arguments for the interactive wizard.
/// Without a terminal, `--name` is required and nothing is written unless
/// `--wetrun` is passed.
// when adding a flag, include it in `has_molt_flags` below
#[derive(FromArgs, Debug)]
pub struct TopLevel {
    /// project name (`snake_case`; used for crate names, headings, and defaults)
    #[argh(option)]
    pub name: Option<String>,

    /// npm package name (defaults to the project name; may be scoped like @you/name)
    #[argh(option)]
    pub npm_name: Option<String>,

    /// one-line project description
    #[argh(option)]
    pub description: Option<String>,

    /// custom domain written to static/CNAME (omit to delete CNAME and homepage)
    #[argh(option)]
    pub domain: Option<String>,

    /// repository url (defaults to the git origin remote when it isn't the template's)
    #[argh(option)]
    pub repo: Option<String>,

    /// features to keep, comma-separated or repeated
    /// (rust, cli, docs, github-extras)
    #[argh(option)]
    pub keep: Vec<String>,

    /// features to strip, comma-separated or repeated
    /// (rust, cli, docs, github-extras)
    #[argh(option)]
    pub strip: Vec<String>,

    /// apply the plan (without it, non-interactive runs write nothing and
    /// the wizard asks before applying)
    #[argh(switch)]
    pub wetrun: bool,

    /// proceed even if the git tree is dirty
    #[argh(switch)]
    pub force: bool,

    #[argh(subcommand)]
    pub subcommand: Option<Subcommand>,
}

impl TopLevel {
    /// Whether any molt-run flag was passed — they're meaningless combined
    /// with the `check` subcommand, so the caller rejects that instead of
    /// silently ignoring them.
    pub const fn has_molt_flags(&self) -> bool {
        self.name.is_some()
            || self.npm_name.is_some()
            || self.description.is_some()
            || self.domain.is_some()
            || self.repo.is_some()
            || !self.keep.is_empty()
            || !self.strip.is_empty()
            || self.wetrun
            || self.force
    }
}

#[derive(FromArgs, Debug)]
#[argh(subcommand)]
pub enum Subcommand {
    Check(CheckCommand),
}

/// Verify molt's anchors and embedded templates still match the template
/// (used by CI and tests).
#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "check")]
pub struct CheckCommand {}
