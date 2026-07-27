# WordPress Archive Viewer

A cross‑platform Tauri desktop application that lets you browse, extract, and run WordPress archive snapshots locally. It ships with built‑in **Caddy**, **PHP‑CGI**, and **MariaDB** runtimes, and automatically manages temporary files.

## Features
- Load a WordPress archive (`.zip` or `.tar.gz`) and view the site locally.
- Extract the archive to a temporary directory or reuse an existing extraction.
- One‑click start/stop of Caddy, PHP‑CGI and MariaDB.
- Cross‑platform binaries are selected at runtime (`.exe` on Windows, no suffix on macOS/Linux).
- Built‑in “Reset temp files” button to recover from corrupted states.

## Getting Started
1. Clone the repository.
2. Install the prerequisites:
   - **Node.js** (v20) and **npm**.
   - **Rust** toolchain (use `rustup`).
   - Platform‑specific dependencies for Tauri (see Tauri docs).
3. Place the appropriate runtime binaries (`caddy`, `php-cgi`, `mariadbd`) in `src-tauri/runtime/` for your OS, or let the GitHub Actions workflow download them.
4. Run the development server:
   ```bash
   npm install
   npm run dev
   ```
5. Build for release:
   ```bash
   npm run tauri:build
   ```

## CI / Cross‑Platform Builds
The repository includes a GitHub Actions workflow (`.github/workflows/build.yml`) that automatically builds the app for **Windows**, **macOS**, and **Linux**, and uploads the bundles as artifacts.

## License
MIT – see `LICENSE` file.
