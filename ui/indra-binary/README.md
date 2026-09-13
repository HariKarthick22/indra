# Native Binary Packages for indra

This directory contains the npm package scaffolding for distributing the
`indra` Rust binary as platform-specific npm packages.

## Packages

| Package | Platform |
|---------|----------|
| `@aaif/indra-binary-darwin-arm64` | macOS Apple Silicon |
| `@aaif/indra-binary-darwin-x64` | macOS Intel |
| `@aaif/indra-binary-linux-arm64` | Linux ARM64 |
| `@aaif/indra-binary-linux-x64` | Linux x64 |
| `@aaif/indra-binary-win32-x64` | Windows x64 |

## Usage

These are platform-specific implementation dependencies and are not intended
to be installed directly. Install `@aaif/indra-acp` instead. It installs the
appropriate package automatically and provides the `indra` command. Each
binary package contains its native executable. Its platform-specific internal
command preserves executable permissions during npm packing;
@aaif/indra-acp remains the sole owner of the supported `indra` command.

## Release preparation

The `.github/workflows/publish-npm.yml` workflow downloads the binaries from an
exact versioned Indra release and prepares the platform package tarballs.
By default it only uploads the verified tarballs as a workflow artifact. Set
the manual `publish` input to publish them through the protected npm production
environment.
