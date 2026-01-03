
# Installation

There are 2 options to install `neptungen`

## Option 1: Install a precompiled binary release

Download the binary from the [github release section](https://github.com/phideg/neptungen/releases) of the neptungen repository.

```shell
curl -L https://github.com/phideg/neptungen/releases/download/v0.11.2/neptungen-v0.11.2-x86_64-unknown-linux-musl.tar.gz > neptungen.tar.gz
```

choose the binary matching your OS. The example above is show the download for linux.

Now unpack the archive location of your preference. On Linux this is typically `/usr/local/bin` and on windows `%LocalAppData%` which should expand to `%SystemDrive%\%Username%\LoggedInUser\AppData\Local`. In order to be sure that the executable will be available on your command line double check that the path to the neptungen executable is part of your `PATH` variable.

## Option 2: Install with cargo

For this to work you need a working [Rust installation](https://rust-lang.org/tools/install/).

```shell
cargo install --git https://github.com/phideg/neptungen.git --tag v0.11.2
```
