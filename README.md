# 50shades (of Graylog)

Graylog REST API client written in Rust.

50shades interfaces with Graylog's query API so that log message lookups can be
performed from the command line. It supports storing logins in native OS
keychains using the `keyring` crate and following up on queries, so that logs
can be viewed in a `tail -f` or `journalctl -f` manner. 50shades unterstands
intuitive English expressions for timespans, courtesy of the excellent
`two_timer` crate.

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
    -c, --config <config>    Path to custom configuration file
    -n, --node <node>        Node to query [default: default]

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

## Default Configuration File

The location of the default configuration file is operating system dependent. To
have it created with sensible values and learn about its location, 50shades
provides the `init` command which prompts for a url, user name and password and
prints the path to the file.

## Password Storage

50shades supports reading passwords from operating system / desktop environment
keyrings, only. Passwords cannot be stored in configuration nor passed or piped
during invocation.


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
