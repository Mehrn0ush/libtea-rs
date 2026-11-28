#!/usr/bin/env python3
"""Make the libtea container image from the two musl release archives.

    make-oci-image.py --version V --revision SHA --amd64 ARCHIVE --arm64 ARCHIVE --out DIR

Writes an OCI image layout to DIR, which must not exist yet: a multi-platform
image (linux/amd64, linux/arm64) whose only file is /libtea, taken unchanged
from libtea-V-x86_64-unknown-linux-musl.tar.gz and
libtea-V-aarch64-unknown-linux-musl.tar.gz. The image runs as 65532:65532 with
/libtea as its entrypoint. Prints the image digest (sha256:...) on stdout.

Every byte is fixed by the inputs: the layer is an uncompressed tar with one
entry (mode 0555, uid/gid 0, no names, mtime 0), the JSON has sorted keys and
no spaces, and nothing records a time or the host. The same archives, version
and revision give the same digest on any machine, so anyone can rebuild the
published image and compare. Standard library only.

It checks the archives' layout and that each binary is an ELF file for its
architecture. It does not check SHA256SUMS or the attestation; the image
workflow does that before running it (docs/design/container-image.md).
"""
import argparse
import hashlib
import io
import json
import os
import re
import sys
import tarfile

VERSION_RE = re.compile(r"^[0-9]+\.[0-9]+\.[0-9]+$")
REVISION_RE = re.compile(r"^[0-9a-f]{40}$")
SOURCE = "https://github.com/Mehrn0ush/libtea-rs"

INDEX = "application/vnd.oci.image.index.v1+json"
MANIFEST = "application/vnd.oci.image.manifest.v1+json"
CONFIG = "application/vnd.oci.image.config.v1+json"
LAYER = "application/vnd.oci.image.layer.v1.tar"

# (OCI architecture, Rust target, ELF e_machine)
PLATFORMS = [
    ("amd64", "x86_64-unknown-linux-musl", 62),
    ("arm64", "aarch64-unknown-linux-musl", 183),
]


def fail(message):
    sys.exit(f"make-oci-image: {message}")


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True).encode()


def binary_from(archive, version, target, machine):
    """The libtea binary from a release archive, after checking its layout."""
    root = f"libtea-{version}-{target}"
    if os.path.basename(archive) != root + ".tar.gz":
        fail(f"{archive} is not named {root}.tar.gz")
    if not os.path.isfile(archive):
        fail(f"{archive} is not a file")
    expected = {root: "dir", f"{root}/LICENSE": "file", f"{root}/README.md": "file", f"{root}/libtea": "file"}
    found = {}
    binary = None
    with tarfile.open(archive, mode="r:gz") as tar:
        for member in tar.getmembers():
            name = member.name.rstrip("/")
            if name in found:
                fail(f"{archive}: {name} appears twice")
            found[name] = "dir" if member.isdir() else "file" if member.isreg() else "other"
            if name == f"{root}/libtea" and member.isreg():
                binary = tar.extractfile(member).read()
    if found != expected:
        fail(f"{archive} holds {sorted(found.items())}, expected {sorted(expected.items())}")
    # ELF, 64-bit, little-endian, and e_machine (offset 18) for this platform.
    if binary[:6] != b"\x7fELF\x02\x01" or int.from_bytes(binary[18:20], "little") != machine:
        fail(f"{archive}: libtea is not a 64-bit little-endian ELF file for {target}")
    return binary


def layer_tar(binary):
    raw = io.BytesIO()
    with tarfile.open(fileobj=raw, mode="w", format=tarfile.USTAR_FORMAT) as tar:
        info = tarfile.TarInfo("libtea")
        info.size = len(binary)
        info.mode = 0o555
        info.mtime = 0
        info.uid = info.gid = 0
        info.uname = info.gname = ""
        tar.addfile(info, io.BytesIO(binary))
    return raw.getvalue()


class Layout:
    def __init__(self, out):
        self.out = out
        self.blobs = os.path.join(out, "blobs", "sha256")
        os.makedirs(self.blobs)

    def blob(self, media_type, data):
        """Store data as a blob and return its descriptor."""
        digest = hashlib.sha256(data).hexdigest()
        with open(os.path.join(self.blobs, digest), "wb") as handle:
            handle.write(data)
        return {"mediaType": media_type, "digest": f"sha256:{digest}", "size": len(data)}

    def file(self, name, data):
        with open(os.path.join(self.out, name), "wb") as handle:
            handle.write(data)


def main(argv):
    parser = argparse.ArgumentParser()
    parser.add_argument("--version", required=True)
    parser.add_argument("--revision", required=True)
    parser.add_argument("--amd64", required=True)
    parser.add_argument("--arm64", required=True)
    parser.add_argument("--out", required=True)
    args = parser.parse_args(argv)
    if not VERSION_RE.match(args.version):
        fail(f"bad version: {args.version!r} (X.Y.Z only, no prerelease)")
    if not REVISION_RE.match(args.revision):
        fail(f"bad revision: {args.revision!r} (a full commit SHA)")
    if os.path.lexists(args.out):
        fail(f"{args.out} already exists")
    archives = {"amd64": args.amd64, "arm64": args.arm64}
    binaries = [
        (arch, binary_from(archives[arch], args.version, target, machine))
        for arch, target, machine in PLATFORMS
    ]

    labels = {
        "org.opencontainers.image.description": "libtea, the Transparency Exchange API (TEA) client, from the attested release archives",
        "org.opencontainers.image.licenses": "Apache-2.0",
        "org.opencontainers.image.revision": args.revision,
        "org.opencontainers.image.source": SOURCE,
        "org.opencontainers.image.title": "libtea",
        "org.opencontainers.image.version": args.version,
    }
    layout = Layout(args.out)
    manifests = []
    for arch, binary in binaries:
        layer = layer_tar(binary)
        config = {
            "architecture": arch,
            "os": "linux",
            "config": {"Entrypoint": ["/libtea"], "User": "65532:65532", "Labels": labels},
            "rootfs": {"type": "layers", "diff_ids": ["sha256:" + hashlib.sha256(layer).hexdigest()]},
        }
        manifest = {
            "schemaVersion": 2,
            "mediaType": MANIFEST,
            "config": layout.blob(CONFIG, canonical(config)),
            "layers": [layout.blob(LAYER, layer)],
        }
        descriptor = layout.blob(MANIFEST, canonical(manifest))
        descriptor["platform"] = {"architecture": arch, "os": "linux"}
        manifests.append(descriptor)
    index = layout.blob(
        INDEX,
        canonical({"schemaVersion": 2, "mediaType": INDEX, "manifests": manifests, "annotations": labels}),
    )
    layout.file("oci-layout", canonical({"imageLayoutVersion": "1.0.0"}))
    tagged = dict(index, annotations={"org.opencontainers.image.ref.name": args.version})
    layout.file("index.json", canonical({"schemaVersion": 2, "mediaType": INDEX, "manifests": [tagged]}))
    print(index["digest"])


if __name__ == "__main__":
    main(sys.argv[1:])
