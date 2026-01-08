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

- [Rust+Cargo](https://rust-lang.org/) (latest stable version)

#### Build Steps

```bash
# Clone the repository
git clone https://github.com/Kaladum/Simple-WC3.git
cd Simple-WC3

# Build in release mode
cargo build --release

# The binary will be located at:
# target/release/simple-wc3.exe (Windows)
# target/release/simple-wc3 (Linux)
```

## Usage

### Hosting a Game

1. Run Simple-WC3
2. Press Enter when prompted (leave the input empty)
3. The application will display a public key/address
4. Share this address with other players who want to join
5. Start WC3
6. Host the game

### Joining a Game

1. Run Simple-WC3
2. When prompted, enter the host's public key/address
3. Press Enter to connect
4. Start WC3
5. Join the Game

## Configuration

The application uses Warcraft 3's default port (6112) for local connections.

If your port is not set to 6112 you will not be able to host!

## Troubleshooting

- Verify that all users use the same Version of Warcraft 3
  - The Game Version can be seen in the bottom right corner of the main menu
  - Warning: Currently only WC3 Frozen Throne Version 1.26.x.x is supported but
    this will change soon
- Verify that all users use the same Version of Simple-WC3
  - Preferably use the latest version
- Look for error messages in the console outputs

## Technical description

The WC3 lobby normally works like this: Each WC3 server broadcasts some `UDP`
packages to PORT `6112` whenever a game is hosted, modified, or closed. Each
client also broadcasts a `SearchForGamesRequest` to `UDP` port `6112` and gets a
`UDP` `SearchForGamesResponse` from each server to the sender port of the
broadcast. The response contains a `TCP` port that the game uses to connect to
the server while joining a game.

This software uses some tricks to mimic this behavior over the internet [*1].

On the server side, this program generates its own `SearchForGamesRequest`
packages and polls the WC3 server with it [*2]. The responses to this are
captured and used to simulate server broadcasts and `SearchForGamesResponses`.
These simulated packages are sent to all connected clients. All real broadcasts
from the local game get ignored.

The client side of this program opens a `random` local `TCP` port and forwards
it to port `6112` on the server. This port is later used for the actual game
connection. All incoming UDP messages from the server are sent to the local
game. They are fake responses to the `SearchForGameRequest` packages that the
WC3 game client is broadcasting into the void. The packages are slightly
modified. The `PORT` is changed to the `random` local `TCP` port and a prefix is
added to the game's name to indicate that this software is used. WC3 thinks that
it has successfully found a game server (without caring about the server running
on localhost). When the player enters a game, the server tries to connect via
`TCP` to the servers `IP` (`localhost`) and the server's port (our random `TCP`
port). All communication on this port is captured and forwarded to port `6112`
on the server.

The game client thinks that the server is running on localhost, and the game
server thinks all clients are also running on its localhost. But everything
works fine.

[*1] => Some workarounds were required because we were not able to find a
reliable way to capture broadcast packages while running on the same machine as
the WC3 instance. Therefore, this software ignores all outgoing and fakes all
incoming broadcasts.

[*2] => Once per second this software sends fake `SearchForGameRequests` to the
WC3 server. The request contains the version of the searching WC3 game and if it
is the normal or Frozen Throne version of the game. To support both games and
multiple versions, a package is sent for every combination of the Game Extension
and the Game Version (1.25 - 1.35). The WC3 server only responds if the
extension and version of the request match its own.

## License

See LICENSE.txt file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
