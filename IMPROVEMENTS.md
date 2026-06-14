# KyberFrog — deferred improvements & tech debt

Things intentionally left out of the current POC, to revisit on their own
branches. Each entry says *what*, *why deferred*, and *how* so we don't forget.

## Client Web UI

### 1. Per-monitor output targeting (needs an upstream kyclient change)
- **What:** let an operator pick *which physical monitor* a kyclient instance
  fullscreens on, from a dropdown of detected monitors.
- **Why deferred:** kyclient can't target an output monitor today. Its
  `set_fullscreen` uses `winit::window::Fullscreen::Borderless(None)` →
  fullscreen on the *current* monitor only. (`--display-idx`/`--display-count`
  are about the **source** display on the server, not the client's output.)
- **How:** in kyclient, enumerate `available_monitors()`, add an
  `--output-monitor <idx>` flag, place the window on that monitor then
  `Fullscreen::Borderless(Some(monitor))`. Then expose the dropdown in the web
  UI. v1 ships with fullscreen-on-current / windowed only.

### 2. Real-time log streaming (SSE) — currently polling
- **What:** stream the client + per-instance kyclient logs live in the UI.
- **Why deferred:** v1 uses simple periodic polling of the last N lines (fine
  for a POC). **Confirmed non-blocking:** poll and SSE share the same source
  (the log files), so this is purely additive.
- **How:** add `GET /logs/stream` (`text/event-stream`) that tails the file and
  pushes new lines; swap the frontend from `setInterval`+`fetch` to
  `EventSource`. The log-reading helper is reused unchanged.

## Auth

### 3. Surface credential management to the user
- **What:** let the operator set per-instance / per-transmitter username +
  password in the UI (client web UI and server tray), instead of the baked-in
  transparent login.
- **Why deferred:** today every config uses the transparent default
  (`vj` / `kyberfrog`, see `DEFAULT_AUTH_*` in `shared`), which is enough for a
  trusted LAN and means the operator types nothing.
- **How:** optional credential fields that, when set, override the transparent
  default in the generated config (server) / the kyclient args (client).

## Misc

### 6. Windows installer (single-click setup)
- **What:** a `KyberFrog-Setup.exe` (Inno Setup) that installs everything to
  `C:\Program Files\KyberFrog\`, adds to PATH, creates Start Menu shortcuts, and
  registers an uninstaller — so operators double-click and go.
- **Why deferred:** prerequisite for real-world deployment; skipped while the
  feature set is still stabilising. Today users must install the Kyber fork
  manually and add it to PATH (see README § Prerequisites).
- **How:** one Inno Setup script bundling both KyberFrog exes + the Kyber fork
  binaries (`kycontroller`, `kyavserver`, `kyclient`) + their MinGW runtime DLLs.
  A checkbox at install time selects "Server (regie)" vs "Client (scene)".

### 4. Embed the tray icon in the exe — ✅ done
- `server/build.rs` and `client/build.rs` embed `kyberfrog.ico` as Windows
  resource ID 1 via `winresource` (calls `windres`). Both trays load it at
  runtime with `GetModuleHandleW` + `LoadImageW(.., 1, IMAGE_ICON, ..)`, falling
  back to a file next to the exe (override) then a stock icon. Deployment is now
  a single self-contained binary per app.

### 5. Server-side runtime control over HTTP ("5b")
- Add/remove/restart transmitters from the server dashboard. **On hold** until we
  pin down the actual need (the value isn't clear yet — to discuss).
