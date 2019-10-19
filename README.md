# 50shades (of Graylog)

[![Latest version](https://img.shields.io/crates/v/fifty-shades)](https://crates.io/crates/fifty-shades)
[![License](https://img.shields.io/crates/l/fifty-shades)](https://www.apache.org/licenses/LICENSE-2.0)
[![CI Status](https://img.shields.io/gitlab/pipeline/cmmc-systems/50shades?gitlab_url=https%3A%2F%2Fgitlab.communicatio.com)](https://gitlab.communicatio.com/cmmc-systems/50shades/pipelines)

[Graylog] REST API client written in Rust.

50shades interfaces with Graylog's query API so that log message lookups can be
performed from the command line. It supports storing logins in native OS
keychains and following up on queries, so that logs can be viewed in a `tail -f`
or `journalctl -f` manner. 50shades unterstands intuitive English expressions
for timespans. Output can be controlled using [Handlebars] syntax.

[Graylog]: https://www.graylog.org/
[Handlebars]: https://handlebarsjs.com/

## Usage

50shades provides several subcommands which come with their own respective sets
of options. Invoking the `help` subcommand on any of the other subcommands, or
passing `--help` to any of the subcommands will print the respective help screen
for that command. Invoking `help` or passing `--help` without a subcommand
prints general help.

```
USAGE:
    50shades [OPTIONS] <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -c, --config <config>        Path to custom configuration file
    -n, --node <node>            Node to query [default: default]
    -t, --template <template>    Template to use for output [default: default]

SUBCOMMANDS:
    follow    Follows the tail of a query (like tail -f on a log file)
    help      Prints this message or the help of the given subcommand(s)
    init      Initializes the configuration file
    login     Stores new password for specified node
    query     Performs one-time query against Graylog
```

Before any actual queries can be performed by either `query` or `follow`,
50shades needs to be supplied with a valid [TOML] configuration file and a
matching table for the specified `node` (defaults to `default`), which has to
consist of a `url` and a `user`.  
A valid configuration file looks like this:

```toml
[nodes.default]
url = 'https://graylog.example.com/api'
user = 'admin'
```

In addition, a matching password has to be stored for the node. This can be done
by invoking 50shades with the `login` command.

[TOML]: https://github.com/toml-lang/toml

### Default Configuration File

The location of the default configuration file is operating system dependent. To
have it created with sensible values and learn about its location, 50shades
provides the `init` command which prompts for a url, user name and password and
prints the path to the file. Initializing the configuration file also writes out
the default output templates which is further explained below.

### Controlling Output

Each query result is output as a single line, controlled by the Handlebars
template referenced by the `--template`, or `-t`, option. 50shades' default
template is specified as follows:

```
[{{default container_name "-"}}] {{{message}}}
```

50shades includes `default` as a custom [helper] which may be used to specify a
default value if a field is missing in a query result. Otherwise, an empty
string would be generated.

[helper]: https://handlebarsjs.com/expressions.html

### Password Storage

50shades supports reading passwords from operating system / desktop environment
keyrings, only. Passwords cannot be stored in configuration nor passed or piped
during invocation.

## Installation

The easiest way to install 50shades is by having a working Rust toolchain
installed and invoking

```
cargo install fifty-shades
```

which will place the resulting binary in `~/.cargo/bin`.

## Copyright

Copyright 2019 Communicatio.Systems GmbH

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
