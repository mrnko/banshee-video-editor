# Changelog

All notable changes are documented here. The project follows Semantic Versioning.

## [0.1.2] - 2026-09-10

### Fixed

- Export maps untouched source audio as an input stream instead of an undefined filter output.
- Blurred-background mode now displays the real synchronized blur layer in the editor preview.

## [0.1.1] - 2026-09-10

### Fixed

- Candidate thumbnails and source-video previews now load through a narrowly scoped Tauri asset protocol.
- Clicking Play opens an actual preview and stops playback at the candidate end time.

## [0.1.0] - 2026-09-10

### Added

- Ukrainian desktop interface with dashboard, projects, library and settings.
- Local-first highlight analysis with optional OpenAI visual ranking.
- Candidate review, trim and internal-range removal, overlay editor and vertical MP4 rendering.
- Local resource library, autosave, expense analytics and release notes.
- Windows Share sheet, Explorer reveal and clipboard path actions after export.
- FFmpeg hardware encoder detection, optional Real-ESRGAN pipeline and safe Lanczos fallback.
- NSIS and portable release configuration.
