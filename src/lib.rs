#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![warn(
    missing_docs,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    rustdoc::missing_crate_level_docs
)]
#![allow(clippy::bool_comparison)]

use std::borrow::Cow;
use std::ffi::{OsStr, OsString};
use std::iter::FusedIterator;
use std::path::{Path, PathBuf};
use std::slice::Iter;

/// Common ground for the three way to look for configuration paths.
#[derive(Debug, Clone, Default)]
pub struct ConfigDirs {
    /// Paths *without* the added app directory/file
    paths: Vec<PathBuf>,

    /// If `true`, the current working directory has already been added
    added_cwd: bool,
    /// If `true`, `$XDG_CONFIG_HOME` (defaulting to `~/.config/`) has already been added
    ///
    /// On Windows this is the AppData/Roaming directory.
    added_platform: bool,
    /// If `true`, `/etc` has already been added
    #[cfg(unix)]
    added_etc: bool,
}

impl ConfigDirs {
    /// Empty list of paths to search configs in.
    ///
    /// ```
    /// use config_finder::ConfigDirs;
    ///
    /// assert!(ConfigDirs::empty().paths().is_empty());
    /// ```
    pub const fn empty() -> Self {
        Self {
            paths: Vec::new(),
            added_cwd: false,
            added_platform: false,
            #[cfg(unix)]
            added_etc: false,
        }
    }

    /// Iterator yielding possible config files or directories.
    ///
    /// # Behaviour
    ///
    /// Will search for `app/base.ext` and `app/base.local.ext`. If the extension is empty, it will
    /// search for `app/base` and `app/base.local` instead.
    ///
    /// Giving an empty `app` or `ext`ension is valid, see examples below.
    ///
    /// # Example
    ///
    /// ```
    /// # fn main() { wrapped(); }
    /// # fn wrapped() -> Option<()> {
    /// use std::path::Path;
    ///
    /// use config_finder::ConfigDirs;
    ///
    /// let mut cd = ConfigDirs::empty();
    /// let mut app_files = cd.add_path("start")
    ///                       .add_path("second")
    ///                       .add_path("end")
    ///                       .search("my-app", "main", "kdl");
    ///
    /// let wl = app_files.next()?;
    /// assert_eq!(wl.path(), Path::new("start/.config/my-app/main.kdl"));
    /// assert_eq!(wl.local_path(), Path::new("start/.config/my-app/main.local.kdl"));
    ///
    /// let wl = app_files.next_back()?;
    /// assert_eq!(wl.path(), Path::new("end/.config/my-app/main.kdl"));
    /// assert_eq!(wl.local_path(), Path::new("end/.config/my-app/main.local.kdl"));
    ///
    /// let wl = app_files.next()?;
    /// assert_eq!(wl.path(), Path::new("second/.config/my-app/main.kdl"));
    /// assert_eq!(wl.local_path(), Path::new("second/.config/my-app/main.local.kdl"));
    ///
    /// assert_eq!(app_files.next(), None);
    /// # Some(()) }
    /// ```
    ///
    /// Without an app subdirectory:
    ///
    /// ```
    /// # fn main() { wrapped(); }
    /// # fn wrapped() -> Option<()> {
    /// use std::path::Path;
    ///
    /// use config_finder::ConfigDirs;
    ///
    /// let mut cd = ConfigDirs::empty();
    /// cd.add_path("start");
    /// let mut app_files =
    ///     cd.add_path("start").search("", "my-app", "kdl");
    ///
    /// let wl = app_files.next()?;
    /// assert_eq!(wl.path(), Path::new("start/.config/my-app.kdl"));
    /// assert_eq!(wl.local_path(), Path::new("start/.config/my-app.local.kdl"));
    ///
    /// assert_eq!(app_files.next(), None);
    /// # Some(()) }
    /// ```
    ///
    /// Without an extension:
    ///
    /// ```
    /// # fn main() { wrapped(); }
    /// # fn wrapped() -> Option<()> {
    /// use std::path::Path;
    ///
    /// use config_finder::ConfigDirs;
    ///
    /// let mut cd = ConfigDirs::empty();
    /// let mut app_files =
    ///     cd.add_path("start").search("my-app", "main", "");
    ///
    /// let wl = app_files.next()?;
    /// assert_eq!(wl.path(), Path::new("start/.config/my-app/main"));
    /// assert_eq!(wl.local_path(), Path::new("start/.config/my-app/main.local"));
    ///
    /// assert_eq!(app_files.next(), None);
    /// # Some(()) }
    /// ```
    #[inline]
    pub fn search(&self, app: impl AsRef<Path>, base: impl AsRef<OsStr>, ext: impl AsRef<OsStr>) -> ConfigCandidates {
        ConfigCandidates::new(&self.paths, app, base, ext)
    }
}

/// Accessors
impl ConfigDirs {
    /// Look at the config paths already added.
    ///
    /// ```
    /// use std::path::PathBuf;
    ///
    /// use config_finder::ConfigDirs;
    ///
    /// let mut cd = ConfigDirs::empty();
    /// assert!(cd.paths().is_empty());
    /// cd.add_path("my/config/path");
    /// assert_eq!(cd.paths(), &[PathBuf::from("my/config/path/.config")]);
    /// ```
    pub fn paths(&self) -> &[PathBuf] {
        &self.paths
    }
}

/// Adding paths to the list
impl ConfigDirs {
    /// Adds `path` to the list of directories to check, if not previously added.
    ///
    /// This path should **not** contain the config directory (or file) passed during
    /// construction.
    ///
    /// # Behaviour
    ///
    /// This function will add `.config` to the given path if it does not end with that
    /// already. This means you can just pass the workspace for your application (e.g. the root of
    /// a git repository) and this type will look for `workspace/.config/<app>`.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::PathBuf;
    ///
    /// use config_finder::ConfigDirs;
    ///
    /// let mut cd = ConfigDirs::empty();
    /// assert!(cd.paths().is_empty());
    /// cd.add_path("my/config/path")
    ///   .add_path("my/other/path/.config"); // .config already present at the end
    /// assert_eq!(cd.paths(), &[
    ///     PathBuf::from("my/config/path/.config"),
    ///     PathBuf::from("my/other/path/.config"), // it has not been added again
    /// ]);
    /// ```
    #[inline]
    pub fn add_path<P: AsRef<Path>>(&mut self, path: P) -> &mut Self {
        self._add_path(path, true)
    }

    /// Adds all the paths starting from `start` and going up until a parent is out of `container`.
    ///
    /// This *includes* `container`.
    ///
    /// If `start` does not [starts with][Path::starts_with] `container`, this will do nothing since
    /// `start` is already out of the containing path.
    ///
    /// # Behaviour
    ///
    /// See [`Self::add_path()`]. This behaviour will be applied to each path added by this method.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::PathBuf;
    ///
    /// use config_finder::ConfigDirs;
    ///
    /// let mut cd = ConfigDirs::empty();
    /// assert!(cd.paths().is_empty());
    /// cd.add_all_paths_until("look/my/config/path", "look/my");
    /// assert_eq!(cd.paths(), &[
    ///     PathBuf::from("look/my/config/path/.config"),
    ///     PathBuf::from("look/my/config/.config"),
    ///     PathBuf::from("look/my/.config"),
    /// ]);
    /// ```
    ///
    /// `"other"` is not a root of `"my/config/path"`:
    ///
    /// ```
    /// use config_finder::ConfigDirs;
    ///
    /// let mut cd = ConfigDirs::empty();
    /// assert!(cd.paths().is_empty());
    /// cd.add_all_paths_until("my/config/path", "other");
    /// assert!(cd.paths().is_empty());
    /// ```
    #[inline]
    pub fn add_all_paths_until<P1: AsRef<Path>, P2: AsRef<Path>>(&mut self, start: P1, container: P2) -> &mut Self {
        fn helper(this: &mut ConfigDirs, start: &Path, container: &Path) {
            start
                .ancestors()
                .take_while(|p| p.starts_with(container))
                .for_each(|p| {
                    this._add_path(p, true);
                });
        }

        helper(self, start.as_ref(), container.as_ref());
        self
    }

    /// Adds the platform's config directory to the list of paths to check.
    ///
    /// |Platform | Value                                 | Example                          |
    /// | ------- | ------------------------------------- | -------------------------------- |
    /// | Unix(1) | `$XDG_CONFIG_HOME` or `$HOME/.config` | `/home/alice/.config`            |
    /// | Windows | `{FOLDERID_RoamingAppData}`           | `C:\Users\Alice\AppData\Roaming` |
    ///
    /// (1): *Unix* stand for both Linux and macOS here. Since this crate is primarily intended for
    /// CLI applications & tools, having the macOS files hidden in `$HOME/Library/Application
    /// Support` is not practical.
    ///
    /// # Behaviour
    ///
    /// This method will **not** add `.config`, unlike [`Self::add_path()`].
    ///
    /// ## Examples
    ///
    /// ```
    /// use std::path::PathBuf;
    ///
    /// use config_finder::ConfigDirs;
    ///
    /// if cfg!(windows) {
    ///     let mut cd = ConfigDirs::empty();
    ///     cd.add_platform_config_dir()
    ///       .add_platform_config_dir(); // Adding twice does not affect the final list
    ///     assert_eq!(cd.paths().len(), 1);
    ///     assert!(cd.paths()[0].ends_with("AppData/Roaming"));
    /// } else {
    ///     std::env::set_var("HOME", "/home/testuser");
    ///
    ///     // With `XDG_CONFIG_HOME` unset
    ///     std::env::remove_var("XDG_CONFIG_HOME");
    ///     let mut cd = ConfigDirs::empty();
    ///     cd.add_platform_config_dir();
    ///     assert_eq!(cd.paths(), &[PathBuf::from("/home/testuser/.config")]);
    ///
    ///     // With `XDG_CONFIG_HOME` set
    ///     std::env::set_var("XDG_CONFIG_HOME", "/home/.shared_configs");
    ///     let mut cd = ConfigDirs::empty();
    ///     cd.add_platform_config_dir();
    ///     assert_eq!(cd.paths(), &[PathBuf::from("/home/.shared_configs")]); // No `.config` added
    /// }
    /// ```
    pub fn add_platform_config_dir(&mut self) -> &mut Self {
        if self.added_platform {
            return self;
        }

        // We don't set `self.added_platform` unconditionnally because the environment can change
        // between the failing call and the next one (which may succeed and then set to true)

        #[cfg(windows)]
        if let Some(path) = dirs_sys::known_folder_roaming_app_data() {
            self._add_path(path, false);
            self.added_platform = true;
        }

        #[cfg(not(windows))]
        if let Some(path) = std::env::var_os("XDG_CONFIG_HOME").and_then(dirs_sys::is_absolute_path) {
            self._add_path(path, false);
            self.added_platform = true;
        } else if let Some(path) = dirs_sys::home_dir().filter(|p| p.is_absolute()) {
            self._add_path(path, true);
            self.added_platform = true;
        }

        self
    }

    /// Adds the current directory to the list of paths to search in.
    ///
    /// # Errors
    ///
    /// Returns an error if [`std::env::current_dir()`] fails.
    ///
    /// # Behaviour
    ///
    /// See [`Self::add_path()`].
    ///
    /// # Examples
    ///
    /// ```
    /// use config_finder::ConfigDirs;
    ///
    /// let current_dir = std::env::current_dir().unwrap().join(".config");
    ///
    /// let mut cd = ConfigDirs::empty();
    /// cd.add_current_dir();
    /// assert_eq!(cd.paths(), &[current_dir]);
    /// ```
    #[inline]
    pub fn add_current_dir(&mut self) -> std::io::Result<&mut Self> {
        if self.added_cwd == false {
            self._add_path(std::env::current_dir()?, true);
            self.added_cwd = true;
        }
        Ok(self)
    }
}

/// Unix-only methods
#[cfg(unix)]
impl ConfigDirs {
    /// Adds `/etc` to the list of paths to checks if not previously added.
    ///
    /// # Behaviour
    ///
    /// This method will **not** add `.config`, unlike [`Self::add_path()`].
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::PathBuf;
    ///
    /// use config_finder::ConfigDirs;
    ///
    /// let mut cd = ConfigDirs::empty();
    /// cd.add_root_etc();
    /// assert_eq!(cd.paths(), &[PathBuf::from("/etc")]);
    /// ```
    #[inline]
    pub fn add_root_etc(&mut self) -> &mut Self {
        if self.added_etc == false {
            self._add_path("/etc", false);
            self.added_etc = true;
        }
        self
    }
}

/// Private methods
impl ConfigDirs {
    /// Helper that will add the `.config` at the end if asked AND if the given path does *not* end
    /// with `.config` already.
    #[inline]
    pub(crate) fn _add_path<P>(&mut self, path: P, check_for_dot_config: bool) -> &mut Self
    where
        P: AsRef<Path>,
    {
        fn helper(this: &mut ConfigDirs, pr: &Path, check_for_dot_config: bool) {
            let path = if check_for_dot_config == false || pr.ends_with(".config") {
                Cow::Borrowed(pr)
            } else {
                Cow::Owned(pr.join(".config"))
            };

            if this.paths.iter().all(|p| p != &path) {
                this.paths.push(path.into_owned());
            }
        }

        helper(self, path.as_ref(), check_for_dot_config);
        self
    }
}

/// Iterator for [`ConfigDirs::search()`].
pub struct ConfigCandidates<'c> {
    conf: WithLocal,
    paths: Iter<'c, PathBuf>,
}

impl<'c> ConfigCandidates<'c> {
    pub(crate) fn new(
        paths: &'c [PathBuf],
        app: impl AsRef<Path>,
        base: impl AsRef<OsStr>,
        ext: impl AsRef<OsStr>,
    ) -> Self {
        Self {
            conf: WithLocal::new(app.as_ref().join(base.as_ref()), ext),
            paths: paths.iter(),
        }
    }
}

impl Iterator for ConfigCandidates<'_> {
    type Item = WithLocal;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let dir = self.paths.next()?;
        Some(self.conf.joined_to(dir))
    }

    #[inline]
    fn last(self) -> Option<Self::Item>
    where
        Self: Sized,
    {
        let dir = self.paths.last()?;
        Some(self.conf.joined_to(dir))
    }

    #[inline]
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        let dir = self.paths.nth(n)?;
        Some(self.conf.joined_to(dir))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.paths.size_hint()
    }

    #[inline]
    fn count(self) -> usize
    where
        Self: Sized,
    {
        self.paths.count()
    }
}

impl DoubleEndedIterator for ConfigCandidates<'_> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        let dir = self.paths.next_back()?;
        Some(self.conf.joined_to(dir))
    }
}

impl ExactSizeIterator for ConfigCandidates<'_> {}

impl FusedIterator for ConfigCandidates<'_> {}

/// Stores both the normal and local form a configuration path.
///
/// The local form has `.local` inserted just before the extension: `cli-app.kdl` has the local form
/// `cli-app.local.kdl`.
///
/// While this is mostly intended for file, nothing precludes an application from using it for
/// directories.
///
/// ```
/// use std::path::{Path, PathBuf};
///
/// use config_finder::WithLocal;
///
/// // `.local` is inserted before the extension for the `.local_path()` form
/// let wl = WithLocal::new("cli-app", "kdl");
/// assert_eq!(wl.path(), Path::new("cli-app.kdl"));
/// assert_eq!(wl.local_path(), Path::new("cli-app.local.kdl"));
///
/// // Even if the extension is empty (can notably be used for directories)
/// let wl = WithLocal::new("cli-app", "");
/// assert_eq!(wl.path(), Path::new("cli-app"));
/// assert_eq!(wl.local_path(), Path::new("cli-app.local"));
///
/// // An empty base is valid too
/// let wl = WithLocal::new("", "kdl");
/// assert_eq!(wl.path(), Path::new(".kdl"));
/// assert_eq!(wl.local_path(), Path::new(".local.kdl"));
///
/// // If you need to store a form (local or not),
/// let wl = WithLocal::new("zellij", "kdl");
/// assert_eq!(wl.into_paths(), (PathBuf::from("zellij.kdl"), PathBuf::from("zellij.local.kdl")));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WithLocal {
    /// The normal path
    path: PathBuf,
    /// The local form of the path.
    local_path: PathBuf,
}

impl WithLocal {
    /// Computes both the normal and local forms of the path.
    ///
    /// If the `ext`ension is non-empty, inserts a dot (`.`) between the `base` and the `ext`ension.
    #[inline]
    pub fn new(base: impl Into<OsString>, ext: impl AsRef<OsStr>) -> Self {
        fn helper(mut path: OsString, ext: &OsStr) -> WithLocal {
            let mut local_path = path.clone();
            local_path.push(".local");

            if ext.is_empty() == false {
                path.push(".");
                path.push(ext);

                local_path.push(".");
                local_path.push(ext);
            }

            WithLocal {
                path: path.into(),
                local_path: local_path.into(),
            }
        }

        helper(base.into(), ext.as_ref())
    }

    /// Path without the added `.local` just before the extension.
    #[inline]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Path with the added `.local` just before the extension.
    #[inline]
    pub fn local_path(&self) -> &Path {
        &self.local_path
    }

    /// Destructure into the inner `(path, local_path)` without allocating.
    #[inline]
    pub fn into_paths(self) -> (PathBuf, PathBuf) {
        (self.path, self.local_path)
    }
}

impl WithLocal {
    // Helper function for the iterator
    fn joined_to(&self, base: &Path) -> Self {
        Self {
            path: base.join(&self.path),
            local_path: base.join(&self.local_path),
        }
    }
}
