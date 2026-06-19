# Schnake

A static Rust/WASM browser game prototype where the first player acts as the
authoritative host. Peers connect directly with WebRTC data channels; the site
does not run a game server.

## What is possible

Browsers cannot expose a normal inbound TCP or UDP listener from WebAssembly.
The practical browser-native shape is:

- Host player loads the static site and runs the simulation in WASM.
- Joining players exchange WebRTC offer/answer data with the host by QR code.
- After the data channel opens, clients send input messages and the host
  broadcasts game state.

This avoids a game server, but it does not avoid the WebRTC handshake.
Automatic matchmaking would still need signaling, and many internet connections
need STUN or TURN to cross NATs. The prototype uses Google's public STUN
endpoint and QR codes for no-server signaling.

## Run

```sh
env -u NO_COLOR trunk serve --address 127.0.0.1 --port 8080
```

Open `http://127.0.0.1:8080/`.

The GitHub Pages deployment is published at
`https://brensch.github.io/schnake/`.

## QR connection flow

1. Host player clicks `Host`.
2. Joining player clicks `Join`, then scans the host QR.
3. Joining player shows the reply QR.
4. Host scans the reply QR.

Camera scanning needs a browser with the `BarcodeDetector` QR API and camera
permission. It also needs a secure page context, such as HTTPS or localhost.

## Build

```sh
env -u NO_COLOR trunk build --release
```

The deployable static site is written to `dist/`.
