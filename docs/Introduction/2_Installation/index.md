
# Installation

There are 2 options to install `neptungen`

## Option 1: Install a precompiled binary release

Download the binary from the [github release section](https://github.com/phideg/neptungen/releases) of the neptungen repository.

```shell
curl -L https://github.com/phideg/neptungen/releases/download/v0.11.2/neptungen-v0.11.2.tar.gz > neptungen.tar.gz
```

Now unpack the archive and move the neptungen executable to a location which is part of your PATH variable. On Linux this is typically `/usr/local/bin` and on windows `%LocalAppData%` which should expand to `%SystemDrive%\%Username%\LoggedInUser\AppData\Local`. In order to be sure double check that the path to the neptungen executable is part of your PATH variable.

## Option 2: Install with cargo

For this to work you need a working [Rust installation](https://rust-lang.org/tools/install/).

```shell
cargo install --git https://github.com/phideg/neptungen.git --tag v0.11.2
```
