# Pinned Package URL test suite

| Field | Value |
| --- | --- |
| Upstream | https://github.com/package-url/purl-spec |
| Release | `v1.1.1` (2026-11-10) |
| Commit | `26671718c3da7225c582a94da319c0c6c3a60cf7` |
| License | MIT (`spec/purl/LICENSE`, unmodified) |

Unmodified copies of the specification and per-type test files, used by
`crates/tea-core/tests/purl_spec_suite.rs` to check `libtea_core::purl`. They
are not packaged into any published crate. Fetched with
`gh api -H 'Accept: application/vnd.github.raw'
repos/package-url/purl-spec/contents/<path>?ref=<commit>`.

The normative text (ECMA-427, `docs/specification/standard/` upstream) is not
vendored. Clause 5 at this commit was read when the parser was written; the
design note `docs/design/purl.md` records what it says and where the suite
and the text disagree.

`crates/tea-core/tests/purl_spec_suite.rs` checks every digest below; a
re-pin replaces the files and this table together.

| Path | Upstream path | SHA-256 |
| --- | --- | --- |
| `spec/purl/LICENSE` | `LICENSE` | `24fb7204fd3c9396c9d83533448cb988e8e86d6f598d1ab51c6d2e5d7e42bcb1` |
| `spec/purl/tests/spec/specification-test.json` | `tests/spec/specification-test.json` | `90ede360478c686b17382d8edfd0447ee37fe92a2ee158caf5d34876c43dbe0d` |
| `spec/purl/tests/types/alpm-test.json` | `tests/types/alpm-test.json` | `f7f5db0274a6bd6c8dd5c0b68728dd60f547b5e71c98eb025f67e483b98cb4f6` |
| `spec/purl/tests/types/apk-test.json` | `tests/types/apk-test.json` | `26e65e022484da623caf341a127f4f7da25242f9b743809923748b9202b2954c` |
| `spec/purl/tests/types/bazel-test.json` | `tests/types/bazel-test.json` | `83815c56654b659dce3c2dc3e902fd2285075080847e54fc488b1c8d5866d0ef` |
| `spec/purl/tests/types/bitbucket-test.json` | `tests/types/bitbucket-test.json` | `893fca5a175c7e7479c8d34ef48aae12e80fce3f45a3655a1450b7940ddfa521` |
| `spec/purl/tests/types/bitnami-test.json` | `tests/types/bitnami-test.json` | `c04eb30ab1f80b44305796b3812b1d7027a57c77d46600b08df8371d2eccd74c` |
| `spec/purl/tests/types/brew-test.json` | `tests/types/brew-test.json` | `f1cf29412fd4765050662f4f6042db9b910c22d06d8680be4d5c9dcbe4ed45f7` |
| `spec/purl/tests/types/cargo-test.json` | `tests/types/cargo-test.json` | `0db52ae5e3774469d2f4a0f26b05f86017e20a43cd36bcc35f1fcc2ffebc9612` |
| `spec/purl/tests/types/chrome-extension-test.json` | `tests/types/chrome-extension-test.json` | `a6247fa31d91a68e51f18216fc09ce931e649ee988ad71d695c38f1400b8d73c` |
| `spec/purl/tests/types/cocoapods-test.json` | `tests/types/cocoapods-test.json` | `93b7006af5a4a3239604b1f01bf157db320e4f7fdf46772d7132fe0657aa4949` |
| `spec/purl/tests/types/composer-test.json` | `tests/types/composer-test.json` | `5beff5a17b364855cef15ad3f7f6ea84dad015c5cd0f1e216790411af5e6168a` |
| `spec/purl/tests/types/conan-test.json` | `tests/types/conan-test.json` | `16bdcbf83e89f9263e2fd9ea6db2add4ac5decf938eff37b1d25ba0acf380287` |
| `spec/purl/tests/types/conda-test.json` | `tests/types/conda-test.json` | `5d7cd36d1312937faab09764f13a6c26b66f7c296bccd96e6b3d4780b5744434` |
| `spec/purl/tests/types/cpan-test.json` | `tests/types/cpan-test.json` | `661843ac4f904c28bf7ee5c592f4dde6eb0b4b07e417c032ff713d2d52475a99` |
| `spec/purl/tests/types/cran-test.json` | `tests/types/cran-test.json` | `78a0914ba348669c91701c61e2efbbf9fcb09d3c9563a5cc4f215fca731483dc` |
| `spec/purl/tests/types/deb-test.json` | `tests/types/deb-test.json` | `5c10a6ce95c6574a4eecb8fa8982d05379033e49224cb9527b15ec08f5e250b4` |
| `spec/purl/tests/types/docker-test.json` | `tests/types/docker-test.json` | `5a7c325aee4d764285036773b1975d6cfc2bb1f9590ad1c1fc3592afc08c7ba0` |
| `spec/purl/tests/types/gem-test.json` | `tests/types/gem-test.json` | `5bacf66e9ea2b2e1ba62ed3be4f120863b678fcc31654452adf1e87dbc5eefd0` |
| `spec/purl/tests/types/generic-test.json` | `tests/types/generic-test.json` | `28ddc17f3de628a0e9f51a6059b8b7a2dbb84c204d3a61e368ee3475fefed748` |
| `spec/purl/tests/types/git-test.json` | `tests/types/git-test.json` | `0679866e38fed12a50ff0cca953ea47314a3938ad6af4bfc011182bbceaf211b` |
| `spec/purl/tests/types/github-test.json` | `tests/types/github-test.json` | `dad6a6832876286cb3f0c84a9708c97fdd43ee2822e25ddde76d00bcbcbde2a8` |
| `spec/purl/tests/types/golang-test.json` | `tests/types/golang-test.json` | `a4939c53cfefcef559f25dbb316b42cb14f28e828ae6c73386bd23304c8d4c3a` |
| `spec/purl/tests/types/hackage-test.json` | `tests/types/hackage-test.json` | `8bef47bc3a3fad9317fc964fee963399b5125f1baf98e74e67c2af5e4b05a0b9` |
| `spec/purl/tests/types/hex-test.json` | `tests/types/hex-test.json` | `72f2a37c8df65f1e9d9b438aafc01855d86ec407edfa570b43338ac000a2aeb6` |
| `spec/purl/tests/types/huggingface-test.json` | `tests/types/huggingface-test.json` | `1cd91a52205cb4ee2a7bde7dfd6d9eeee06ac089a7a3a665893d2f2e5b1b9217` |
| `spec/purl/tests/types/julia-test.json` | `tests/types/julia-test.json` | `08e20b714ce6be59ee61f719bb2fc221d8c2cafddb372dce53e2b33f9ac58b1b` |
| `spec/purl/tests/types/luarocks-test.json` | `tests/types/luarocks-test.json` | `7b31bccb209ac3857606ae6fe2416cd31d0ad703bc2b8eadf86f1f10c527623f` |
| `spec/purl/tests/types/maven-test.json` | `tests/types/maven-test.json` | `035ceec0bfc75a4f3b5c63af8fb88e4e5201a6dd1734f3771bff211bf9ee7d0c` |
| `spec/purl/tests/types/mlflow-test.json` | `tests/types/mlflow-test.json` | `d2e59ab361dfe5a002ef3f83781236b20ff98fdfbcd3d8c1c1afca8a92682013` |
| `spec/purl/tests/types/npm-test.json` | `tests/types/npm-test.json` | `c12d4542b379988456a96fb6082bd53c638d74283299b3d31a65d7509fa0883c` |
| `spec/purl/tests/types/nuget-test.json` | `tests/types/nuget-test.json` | `68521f49c8d7ad29150257914f75082dc2b40ec22b996ed8b61db2b5c9312218` |
| `spec/purl/tests/types/oci-test.json` | `tests/types/oci-test.json` | `07050066f265cf7e7c420bc6a237eb1301a081eb73c4a009ff2f787c95f7974f` |
| `spec/purl/tests/types/opam-test.json` | `tests/types/opam-test.json` | `cf2565d68d392a3fd10f8da1c77beefccae04f275ed5a68116c20305c459c3ed` |
| `spec/purl/tests/types/otp-test.json` | `tests/types/otp-test.json` | `bb73b1b733acae7f531b69d6137de214cea56143430422579f91943d48bb51bc` |
| `spec/purl/tests/types/pub-test.json` | `tests/types/pub-test.json` | `b4469c01c2d4a210bea64eddac55b02825214162aa55bee210832678fd45c9c9` |
| `spec/purl/tests/types/pypi-test.json` | `tests/types/pypi-test.json` | `eecaa56d704cfad8c6da46646d674ae471d1e62c1e5afa335b746ce33be2b7af` |
| `spec/purl/tests/types/qpkg-test.json` | `tests/types/qpkg-test.json` | `e2b9f2b92292c6441fb41d543d86524c53906b073c1db48f9767c1f33c566463` |
| `spec/purl/tests/types/rpm-test.json` | `tests/types/rpm-test.json` | `6923ef3de233df394e6fdba17c06ba1ae668ad114ed4ead658ce2eff4c5d8b8f` |
| `spec/purl/tests/types/swid-test.json` | `tests/types/swid-test.json` | `0b7f6fdfb3fc0599fcc5e731a31fd1349b1df01fd5a6865b4f9c99f35ddc2d71` |
| `spec/purl/tests/types/swift-test.json` | `tests/types/swift-test.json` | `dcc772ed33fa3b58846c89f2a1e0bd526d4b099a85dc92704c983af8f1d55973` |
| `spec/purl/tests/types/vcpkg-test.json` | `tests/types/vcpkg-test.json` | `da8c9b02666e38a8a61612c532b2b39c414d5dd18d9770a3f34a391cbce7acf7` |
| `spec/purl/tests/types/vscode-extension-test.json` | `tests/types/vscode-extension-test.json` | `23f9413163d7d5c3ffece71dc995e44694a6545468062a49b01109e5bc319d3d` |
| `spec/purl/tests/types/yocto-test.json` | `tests/types/yocto-test.json` | `f450741ae6b5ea0e64add94c61ce2a69b378ee32c9d6f25cb57081981904b8f0` |
