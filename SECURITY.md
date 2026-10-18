# Security

## Threat model in one paragraph

Driverix runs elevated to install drivers, which makes it a privileged bridge between a webview and the
driver store. The webview is treated as untrusted: it can only name update IDs and backup IDs that the
backend itself produced, and the backend re-validates them (`src-tauri/src/guard.rs`). Drivers come only from
Windows Update, which verifies Microsoft signatures. The app never downloads or runs executables itself.

## What it will not do

- Install anything unsigned or from a source other than Windows Update.
- Auto-accept EULAs, bypass the 24 h restore-point limit through the registry, or reboot on its own.
- Send telemetry. The only network traffic is the Windows Update Agent's own.

## Reporting

Open a private security advisory on the repository, or email the maintainer. Please include the Windows
build, the Driverix version and whether the issue needs elevation.
