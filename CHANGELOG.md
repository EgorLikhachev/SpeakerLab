# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned

- Baffle-step / diffraction options
- PNG export of graphs
- Bundled driver database

## [0.2.0] - 2026-08-26

### Added

- Voltage limits: maximum voltage/power before exceeding Xmax, port air
  velocity, or thermal rating (WinISD-style), shown in the summary bar
  with a warning when the current generator voltage exceeds the limit.
- Meaningful summary metrics: \|Z\| max searched in 15–500 Hz (system
  resonance, not the HF inductance rise), excursion max in 15–300 Hz,
  and a new "excursion at tuning" metric.
- LR-2 semi-inductance voice-coil model (`Z = Re + Kes·√(jω)`) with a
  +3 dB/oct HF impedance slope, closer to real drivers than the ideal
  +6 dB/oct inductance; switchable per driver.
- UI persistence: window size, language, active graph tab, and generator
  voltage are remembered between launches.
- Driver library search/filter with a visible counter.
- "Port mismatch" warning when the applied port was designed for a
  different Fb than the current tuning.
- Error toasts (5-second popups) instead of invisible stderr messages.
- README screenshots (main view, port calculator, box dimensions,
  TL editor, driver library).
- CI: independent Python verification (20 checks) runs on every push;
  MSRV job; release workflow building Windows/Linux/macOS binaries.

### Fixed

- Library save failed for driver names containing quotes or other
  characters that are illegal in Windows file names.
- Units are now fully localized in the English interface (Hz, Ω, mm, L…).
- V-limit chip showed empty watts; box-dimensions line missed the ×
  separator and duplicated the unit.

### Changed

- MSRV raised to 1.88 (the dependency graph requires it).

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

[Unreleased]: https://github.com/EgorLikhachev/SpeakerLab/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/EgorLikhachev/SpeakerLab/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/EgorLikhachev/SpeakerLab/releases/tag/v0.1.0
