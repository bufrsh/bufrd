bufrd is the client daemon for bufr.sh, intended to run on Linux devices. It is
distributed as a completely static binary, so it should run on most kinds of
Linux devices.

bufrd is built on the following principles:
- Never reads or writes to any file except these:
	- bufr.conf: the only configuration file with activity lines, login
	information (username and hashed password), and the name of device set during
	installation.
	- key.pem: SSL private key, generated during installation by the daemon.
	- cert.pem: SSL certificate, sent by server when the daemon is run for the
	first time.
- Never send server the actual bash commands of activities. Only names and
schedules must be sent.
- Fully encrypted communication with the server.
- Can only run activities that are configured. In other words, it is impossible
for you (or us), to trigger an arbitrary bash command remotely.
- No security vulnerabilities. It is written in Rust lang, which helps us
achieve this easily.
- No `unsafe` code blocks.
- Fully static binary with no dependencies, not even libc.

## Compile
The release versions are completely statically linked. The following steps are
taken to generate them:
```
# install MUSL
sudo apt-get update musl musl-dev musl-tools
CC=musl-gcc cargo build --release --target=$TARGETARCH
strip target/$TARGETARCH/release/bufrd
```
`$TARGETARCH` is `arvm7-unknown-linux-musleabihf` for a RaspberryPi running
Raspbian OS, and `x86_64-unknown-linux-musl` for a Linux PC. More architectures
will be released as they are tested.

## Install
You can compile the binary yourself using Rust stable toolchain, or you can
get it from the latest release section of this Github repo.

The binary can be placed anywhere. Once you have the binary in place, run
`./bufrd gen`. When asked, assign a device name and give your bufr.sh account
info. Everything will be stored in `bufr.conf` file in the same directory along
with a couple of sample activities. You can run `./bufrd` as a standalone, or
set it up as a systemd service. When it runs for the first time, the daemon
generates `key.pem` and downloads a signed `cert.pem` file from the server.

## Bug reports
Please file any issues that you find. Pull requests are closed for now.

## License
No license for now.

