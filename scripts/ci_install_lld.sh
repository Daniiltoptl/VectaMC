#!/usr/bin/env bash
# Makes `ld.lld` available on Linux CI runners for `-fuse-ld=lld`.
# apt mirrors on the runner image sometimes time out, which used to fail the
# whole job before any Rust step ran. Try apt (tolerating a partial update),
# then fall back to the `rust-lld` that ships with the Rust toolchain.
set -u

if command -v ld.lld >/dev/null 2>&1; then
  echo "ld.lld already present: $(command -v ld.lld)"
  exit 0
fi

sudo apt-get -o Acquire::Retries=3 -o Acquire::http::Timeout=20 update || true
if sudo apt-get -o Acquire::Retries=3 install -y --no-install-recommends lld; then
  command -v ld.lld && exit 0
fi

host=$(rustc -vV | sed -n 's/^host: //p')
wrapper="$(rustc --print sysroot)/lib/rustlib/${host}/bin/gcc-ld/ld.lld"
if [ -x "$wrapper" ]; then
  sudo ln -sf "$wrapper" /usr/local/bin/ld.lld
  echo "Using rust-lld wrapper: $wrapper"
  ld.lld --version
  exit 0
fi

echo "::error::Could not install lld from apt and no rust-lld wrapper at $wrapper"
exit 1
