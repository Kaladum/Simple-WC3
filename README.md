# Simple-WC3

A unofficial [Warcraft 3](https://warcraft3.blizzard.com/) LAN connectivity tool
built with Rust, enabling LAN Games over the internet using the
[Iroh](https://www.iroh.computer/) network protocol.

This library is in a pre-release version. First tests have been successful but
there a still some bugs and problems left.

## Features

- Host or join [Warcraft 3](https://warcraft3.blizzard.com/) games over the
  internet
- Peer-to-peer connectivity using [Iroh](https://www.iroh.computer/)
- Cross-platform support (Windows and Linux)
- No installation, registration, port forwarding, VPN, ... required

## Installation

### Pre-built Binaries

This is a portable app. No installation required.

Jus download the latest release for your platform from the **Assets** section on
the [Releases](https://github.com/Kaladum/Simple-WC3/releases/latest) page:

- **Windows**: `simple-wc3-windows-x86_64.exe`
- **Linux**: `simple-wc3-linux-x86_64`

### Building from Source

#### Prerequisites

- [Rust](https://rustup.rs/) (latest stable version)
- Cargo (included with Rust)

#### Build Steps

```bash
# Clone the repository
git clone <repository-url>
cd wc-lan

# Build in release mode
cargo build --release

# The binary will be located at:
# target/release/simple-wc3.exe (Windows)
# target/release/simple-wc3 (Linux)
```

## Usage

### Hosting a Game

1. Run the application
2. Press Enter when prompted (leave the input empty)
3. The application will display a public key/address
4. Share this address with other players who want to join

### Joining a Game

1. Run the application
2. When prompted, enter the host's public key/address
3. Press Enter to connect

## Configuration

The application uses Warcraft 3's default port (6112) for local connections.

If your port is not set to 6112 you will not be able to host!

## License

See LICENSE.txt file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
