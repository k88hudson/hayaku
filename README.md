# hayaku

Hayaku is a simple tool for quickly generating new projects from
template directories and simple prompts.

## Getting Started

```bash
cargo install hayaku
```

Once installed, run the initializer to create your local template directory
and global configuration file:

```bash
hayaku init
```

This command creates (or confirms) the default template location
`~/.hayaku`. You can change the location of this directory by setting
`HAYAKU_DIRECTORY` environment variable in your shell.

You can also add global environment variables for use in templates to
`~/.hayaku/hayaku.settings.toml`. For example:

```toml
[global_env]
author = "k88hudson"
```

## Commands

Run `hayaku --help` to see a list of all commands and options.

- `hayaku create <path>` — generate a project from either a local or built-in template.
  The command walks you through selecting a template and entering destination
  details.
- `hayaku list` — show which templates are currently available, organized into
  local and built-in sources.

## Creating templates

Hayaku templates are just directories with files and folders. The default location
for local templates is `~/.hayaku/templates`, but you can adjust this with the
`HAYAKU_TEMPLATES_DIR` environment variable in your shell.

For example:

```
.hayaku/templates/
    rust/
        hayaku.toml
        Cargo.toml
        src/
            main.rs
```

### Variables

Files can contain variables and control flow logic using [Tera](https://tera.netlify.app/docs/) templating.

The `{{ PROJECT_NAME }}` variable is available to all templates. For example,
you could use this in a`Cargo.toml` file:

```toml
[package]
name = "{{ PROJECT_NAME }}"
version = "0.1.0"
edition = "2021"
```

You can also use variables in the paths of filenames, surrounded by `[` and `]`.
For example, if you run `hayaku create my_project` with a template that contains the file:

```
init_[PROJECT_NAME].rs
```

the generated file will be named `init_my_project.rs`.

### Configuration

If you want, you can add a `hayaku.toml` file to the root of your template
directory. This file can contain some metadata like the name and description,
and also specify custom variables that can extend the create prompt.

Here's an example:

```toml
[template]
name = "rust"

[env.crate_type]
type = "choices"
prompt = "Do you want a library or binary crate?"
choices = ["lib", "bin"]
default = "bin"

[env.workspace]
type = "bool"
prompt = "Is this a workspace?"

[env.author]
type = "string"
prompt = "What is your name?"
```

Variables are converted to uppercase:

```toml
[package]
name = "{{ PROJECT_NAME }}"
version = "0.0.0"
author = "{{ AUTHOR | default(value='') }}"
edition = "2024"
license = "{{ LICENSE | default(value='MIT') }}"
```

Note that you can define global variables in `hayaku.settings.toml`, which
will be available to all templates. In the example above, if you had defined
`license` in your global settings, it would be used here.
