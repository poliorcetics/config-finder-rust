# Config finder

Automatically search for `<dirs>/.config/my-app` for your application, including local-only files.

See [`ConfigDirs`] for the entry point.

## The problem

Following the "show the door before showing the key" adage, following is an explanation of why
this crate exists.

Our imaginary CLI application, let's name it "Pear", transparently connects directories from
our local machine to a remote server, keeping them in sync.

It has a global config, applied to all connections, but also needs directory-specific ones, for
example `dir-perso` points to `server-perso` and `dir-work` points to `server-work`. The company
uses a default configuration to point to the work server and developers are expected to use
their own credentials to connect.

Because "Pear" is a nice app following good CLI practices, it puts repository-scoped config in
`repo/.config/pear/...` files.

So the structure is as follow:

```text
$HOME
|- .config/ (or where $XDG_CONFIG_HOME is pointing to)
|  +- pear/
|     +- config.kdl
|
|- perso/project-A
|  |- .config/
|  |  +- pear/
|  |     +- config.kdl
|  |- Cargo.toml
|  +- src/..
|
+- work/
   |- .config/
   |  +- pear/
   |     |- config.kdl
   |     +- config.local.kdl
   |- Cargo.toml
   +- src/..
```

**How do we find all the config files nicely ?**

There is `$XDG_CONFIG_HOME`, `$HOME/.config` if the former is unset, `.config` dirs in various
repos, `config.local.kdl` vs `config.local`, it's hard to find everything and not forget one.

## The solution

Obviously, you're on the documentation for a `config-finder` crate, so it's the (one) solution.

Here's an example for the work config:

```rust
# fn main() { wrapped(); }
# fn wrapped() -> Option<()> {
use std::path::Path;

use config_finder::ConfigDirs;

std::env::set_var("XDG_CONFIG_HOME", "/configs/user-1");

let mut cd = ConfigDirs::empty();
let mut files = cd.add_path("~/work") // `.config` is automatically added
                  .add_platform_config_dir() // `.config` is not added for this
                  // Takes a reference to the original `ConfigDirs` so you can create
                  // multiple iterators to search for multiple files or directories
                  .search("pear", "config", "kdl");

let with_local = files.next()?;
assert_eq!(with_local.path(), Path::new("~/work/.config/pear/config.kdl"));
assert_eq!(with_local.local_path(), Path::new("~/work/.config/pear/config.local.kdl"));

let with_local = files.next()?;
if cfg!(windows) {
   assert_eq!(with_local.path(), Path::new(r"C:\Users\runneradmin\AppData\Roaming\pear\config.kdl"));
   assert_eq!(with_local.local_path(), Path::new(r"C:\Users\runneradmin\AppData\Roaming\pear\config.local.kdl"));
} else {
   assert_eq!(with_local.path(), Path::new("/configs/user-1/pear/config.kdl"));
   assert_eq!(with_local.local_path(), Path::new("/configs/user-1/pear/config.local.kdl"));
}

assert_eq!(files.next(), None);
# Some(()) }
```

## Further details

What your app does next with the local and normal form of the config files and directories is
entirely up to you. You can merge local and normal config, entirely ignore the normal one if
a local one is present, only accept non-local configs, it's your choice.

Absolutely no checking is done in the given paths since this library cannot know about the shape
of your system nor the requirements of your application.

Canonicalization of paths is not done either since it requires filesystem access. If you want to
use only canonicalized paths, wrap the types exposed by this library.

## MSRV

Current MSRV is **Rust 1.56.0**. I don't expect this to change much over time, but if need be, it
will be done in a minor version (at least) and will not move further than 3 versions back (e.g.,
if current Rust version is 1.65, it will not jump to later than 1.62).