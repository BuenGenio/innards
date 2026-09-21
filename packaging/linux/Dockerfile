# Tauri 2 Linux build box: Ubuntu 22.04 (glibc 2.35) so .deb/.rpm/.AppImage run on anything newer.
FROM ubuntu:22.04
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential curl wget file ca-certificates git pkg-config xz-utils \
    libwebkit2gtk-4.1-dev libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev patchelf \
    squashfs-tools libfuse2 desktop-file-utils gettext xdg-utils \
  && rm -rf /var/lib/apt/lists/*
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
  && apt-get install -y --no-install-recommends nodejs && rm -rf /var/lib/apt/lists/* \
  && npm install -g pnpm@10
ARG UID=1000
ARG GID=1000
RUN groupadd -g $GID build && useradd -m -u $UID -g $GID build
USER build
ENV PATH=/home/build/.cargo/bin:$PATH \
    APPIMAGE_EXTRACT_AND_RUN=1 \
    CARGO_TERM_COLOR=never
RUN curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable
WORKDIR /work
