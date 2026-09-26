# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.0] - 2026-09-26

### Added
- Assigns lightguns to Player 1/2 in the Supermodel INI by USB VID:PID
- `--list` shows all connected devices with their `MOUSEx` index
- `--use-analog` writes the analog gun settings (Ocean Hunter, LA Machineguns)
- Windows: mice are enumerated via Raw Input in Supermodel's order, so the `MOUSEx` indices match
- CI/CD pipeline with GitFlow releases: `release-start.yml` creates release/hotfix branches, `release-finish.yml` tags, builds and publishes GitHub Releases
- SemVer versioning via `scripts/version.sh`, injected at build time (unofficial builds are `0.0.0+<sha>`)
