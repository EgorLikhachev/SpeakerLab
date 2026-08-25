# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned

- LR-2 lossy voice-coil inductance model (more accurate high-frequency rolloff)
- Baffle-step / diffraction options
- PNG export of graphs
- Bundled driver database

## [0.1.0] - 2026-08-25

Initial public release.

### Added

- Enclosure types: sealed box, bass-reflex, passive radiator, bandpass
  4th/6th order, transmission line / quarter-wave resonator / horn
  (segmented lossy-line model with stuffing).
- Live recalculation: every graph updates in the same frame as any
  parameter change.
- Graphs with log frequency axis and zoom: SPL, impedance magnitude,
  impedance phase, cone excursion (with Xmax line), port air velocity
  (with 17/22 m/s limits), group delay.
- Port calculator (length ⇄ Fb, round/slot, multiple ports, end
  corrections, air-velocity check).
- Box dimension calculator (net/gross volume, displacements, proportions,
  panel area).
- Alignment suggestions from T/S parameters (Qtc targets, flat/compact/EBS
  vented tunings, passive-radiator mass).
- Reference-curve comparison (dashed overlay on all graphs).
- Project files `.spkproj` and a personal driver library in JSON.
- CSV export of all calculated curves.
- User interface in Russian and English with live switching.
- Physics core with 32 unit tests and an independent Python cross-
  verification (14 checks: independent circuit re-implementation,
  textbook closed-form formulas, qualitative physics signatures).

[Unreleased]: https://github.com/EgorLikhachev/SpeakerLab/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/EgorLikhachev/SpeakerLab/releases/tag/v0.1.0
