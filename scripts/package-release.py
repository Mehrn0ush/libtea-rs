#!/usr/bin/env python3
"""Package the libtea binary for a release, deterministically.

    package-release.py --target T --version V --binary PATH --out DIR [--files F ...]

Writes DIR/libtea-V-T.tar.gz (or .zip when T contains "windows") holding
`libtea` (`libtea.exe`), LICENSE and README.md under a `libtea-V-T/` directory.
Entries are sorted, owned by uid/gid 0 with empty names, and carry a fixed
mtime, so the same inputs give the same archive bytes. The gzip header carries
no name or timestamp. Prints the archive path on stdout.

This does not sign anything and does not make the build reproducible; it only
removes archive-level noise.
"""
import argparse
import gzip
import io
import os
import re
import stat
import sys
import tarfile
import zipfile

FIXED_MTIME = 0
TARGET_RE = re.compile(r"^[A-Za-z0-9_]+(-[A-Za-z0-9_]+){2,3}$")
VERSION_RE = re.compile(r"^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.]+)?$")


def read(path):
    with open(path, "rb") as handle:
        return handle.read()


def entries(root, binary_name, binary, extra):
    """(archive name, bytes, mode) sorted by name."""
    items = [(f"{root}/{binary_name}", read(binary), 0o755)]
    for path in extra:
        items.append((f"{root}/{os.path.basename(path)}", read(path), 0o644))
    return sorted(items, key=lambda item: item[0])


def make_tar_gz(path, root, items):
    raw = io.BytesIO()
    with tarfile.open(fileobj=raw, mode="w", format=tarfile.PAX_FORMAT) as tar:
        directory = tarfile.TarInfo(root + "/")
        directory.type = tarfile.DIRTYPE
        directory.mode = 0o755
        directory.mtime = FIXED_MTIME
        tar.addfile(directory)
        for name, data, mode in items:
            info = tarfile.TarInfo(name)
            info.size = len(data)
            info.mode = mode
            info.mtime = FIXED_MTIME
            info.uid = info.gid = 0
            info.uname = info.gname = ""
            tar.addfile(info, io.BytesIO(data))
    with open(path, "wb") as out:
        # filename="" and mtime=0 keep the gzip header free of host data.
        with gzip.GzipFile(filename="", mode="wb", fileobj=out, mtime=0, compresslevel=9) as gz:
            gz.write(raw.getvalue())


def make_zip(path, root, items):
    with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
        for name, data, mode in items:
            info = zipfile.ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.create_system = 3
            info.external_attr = (stat.S_IFREG | mode) << 16
            archive.writestr(info, data)


def main(argv):
    parser = argparse.ArgumentParser()
    parser.add_argument("--target", required=True)
    parser.add_argument("--version", required=True)
    parser.add_argument("--binary", required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--files", nargs="*", default=["LICENSE", "README.md"])
    args = parser.parse_args(argv)
    if not TARGET_RE.match(args.target):
        sys.exit(f"bad target: {args.target!r}")
    if not VERSION_RE.match(args.version):
        sys.exit(f"bad version: {args.version!r}")
    windows = "windows" in args.target
    binary_name = "libtea.exe" if windows else "libtea"
    root = f"libtea-{args.version}-{args.target}"
    items = entries(root, binary_name, args.binary, args.files)
    os.makedirs(args.out, exist_ok=True)
    path = os.path.join(args.out, root + (".zip" if windows else ".tar.gz"))
    (make_zip if windows else make_tar_gz)(path, root, items)
    print(path)


if __name__ == "__main__":
    main(sys.argv[1:])
