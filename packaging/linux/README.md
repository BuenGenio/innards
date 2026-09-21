# Linux bundles (.deb / .AppImage / .rpm)

Built inside an Ubuntu 22.04 container (glibc 2.35) so the output runs on anything newer; no host packages needed beyond Docker.

```
./packaging/linux/build.sh      # expects the repo at ~/Projects/innards; output in target/release/bundle/{deb,rpm,appimage}/
```

`build.sh` builds the image (`innards-tauri-build`, ~1.8 GB) once, then runs `pnpm install --frozen-lockfile && pnpm tauri build` as your uid with `~/.cargo/registry` and `~/.cache/tauri` mounted, so rebuilds are incremental. `APPIMAGE_EXTRACT_AND_RUN=1` lets linuxdeploy run without FUSE; `xdg-utils` is required by tauri-plugin-opener at bundle time.

Publish: `gh release create vX.Y.Z --target main --title "Innards X.Y.Z" --notes-file notes.md target/release/bundle/deb/*.deb target/release/bundle/appimage/*.AppImage target/release/bundle/rpm/*.rpm SHA256SUMS`. The `web/` download page picks the assets up from the GitHub releases API by extension; `site/public/download.html` links to the files by name and must be updated per release.

v0.1.0 was built this way on Stormshadow (20 cores, ~4 min cold).
