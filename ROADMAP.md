# Roadmap

## v0.1 — Core (Complete)

- [x] Project foundation & developer tooling
- [x] SQLite database layer (schema, migrations, query layer)
- [x] File scanning engine (directory traversal, metadata collection)
- [x] Duplicate detection (BLAKE3 multi-stage: size → partial hash → full hash)
- [x] Results UI (master-detail layout, duplicate groups display)
- [x] Selection & safe deletion (smart selection, batch trash, deletion history)
- [x] Code review & hardening (98-finding review, 8 high-severity fixes, test safety net)

## v0.2 — Polish (Next)

- [ ] Scan progress display & ETA estimation
- [ ] Pause/resume scan controls
- [ ] Settings & protected folders (theme, parallelism, folder protection)
- [ ] File operations (open, reveal in Finder/Explorer, copy path)
- [ ] Filtering & search (file type filters, search, image thumbnails)
- [ ] Keyboard navigation & accessibility

## v0.3 — Production Ready

- [ ] Permission wizard (macOS Full Disk Access)
- [ ] Error handling & recovery (skip/retry, disk full, adaptive I/O throttling)
- [ ] Platform polish, Windows support & E2E tests
- [ ] System tray integration (minimize to tray, progress tooltip)

> **Note**: Development currently targets **macOS**. Windows support (including Windows-specific permissions wizard, native styling, and testing) is planned for the platform polish stage (v0.3).

---

See [`docs/plans/00-index.md`](docs/plans/00-index.md) for detailed implementation plans.
