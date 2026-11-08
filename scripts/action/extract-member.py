#!/usr/bin/env python3
"""Extract exactly one regular-file member from a .tar.gz or .zip archive.

    extract-member.py ARCHIVE MEMBER DEST [--max-bytes N]

Nothing else in the archive is read or written. MEMBER must be the exact
archive path (no wildcard); links, directories and special files are refused,
as is a member larger than --max-bytes (default 256 MiB). DEST is created with
mode 0755. Exit status 0 on success, 1 on any refusal.
"""
import os
import stat
import sys
import tarfile
import zipfile

DEFAULT_MAX = 256 * 1024 * 1024


def fail(message):
    sys.stderr.write(f"extract-member: {message}\n")
    sys.exit(1)


def copy_limited(source, dest, limit):
    written = 0
    tmp = dest + ".part"
    with open(tmp, "wb") as out:
        while True:
            chunk = source.read(1 << 20)
            if not chunk:
                break
            written += len(chunk)
            if written > limit:
                out.close()
                os.unlink(tmp)
                fail("member exceeds the size limit")
            out.write(chunk)
    os.chmod(tmp, 0o755)
    os.replace(tmp, dest)


def main(argv):
    limit = DEFAULT_MAX
    if "--max-bytes" in argv:
        index = argv.index("--max-bytes")
        limit = int(argv[index + 1])
        argv = argv[:index] + argv[index + 2:]
    if len(argv) != 3:
        fail("usage: extract-member.py ARCHIVE MEMBER DEST [--max-bytes N]")
    archive, member, dest = argv
    if member.startswith("/") or ".." in member.split("/"):
        fail("refusing a member path that is absolute or contains ..")
    if archive.endswith(".zip"):
        with zipfile.ZipFile(archive) as z:
            try:
                info = z.getinfo(member)
            except KeyError:
                fail("member not found in the archive")
            mode = (info.external_attr >> 16) & 0o170000
            if info.is_dir() or (mode and not stat.S_ISREG(mode)):
                fail("member is not a regular file")
            if info.file_size > limit:
                fail("member exceeds the size limit")
            with z.open(info) as source:
                copy_limited(source, dest, limit)
    else:
        with tarfile.open(archive, "r:gz") as t:
            try:
                info = t.getmember(member)
            except KeyError:
                fail("member not found in the archive")
            if not info.isreg():
                fail("member is not a regular file")
            if info.size > limit:
                fail("member exceeds the size limit")
            source = t.extractfile(info)
            if source is None:
                fail("member cannot be read")
            copy_limited(source, dest, limit)


if __name__ == "__main__":
    main(sys.argv[1:])
