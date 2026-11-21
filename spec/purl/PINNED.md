# Pinned Package URL test suite

| Field | Value |
| --- | --- |
| Upstream | https://github.com/package-url/purl-spec |
| Release | `v1.1.1` (2026-11-10) |
| Commit | `26671718c3da7225c582a94da319c0c6c3a60cf7` |
| License | MIT (`spec/purl/LICENSE`, unmodified) |

Unmodified copies of the specification and per-type test files, and of the
per-type definitions (`types/*-definition.json`, read for their
`case_sensitive`, `requirement`, `permitted_characters` and required qualifier
fields), used by
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
| `spec/purl/types/alpm-definition.json` | `types/alpm-definition.json` | `004da4440f57555ad3b66f0a6e8f999a9166f8b0c8255d0f396a53bd90322141` |
| `spec/purl/types/apk-definition.json` | `types/apk-definition.json` | `3275dc59a544252841220dce0de16c5ba312637312a212c0d83e6f351f6a0c47` |
| `spec/purl/types/bazel-definition.json` | `types/bazel-definition.json` | `b0bd498eb1893d9db13cc4adcc6b9d6e98b9a1a4397835ff731e9fe9c93e3b81` |
| `spec/purl/types/bitbucket-definition.json` | `types/bitbucket-definition.json` | `211660d7d7772a8992958f0c9fa6b089d959ca70500420124f2529570d3a5a51` |
| `spec/purl/types/bitnami-definition.json` | `types/bitnami-definition.json` | `2936f485be51880640ca0b54189c845aa5865cda586408162b5e70f42595977b` |
| `spec/purl/types/brew-definition.json` | `types/brew-definition.json` | `fd330acdb9b2970e1ece91748c0b126039465b87d33197d83e32e023f2fc6725` |
| `spec/purl/types/cargo-definition.json` | `types/cargo-definition.json` | `6dd6cc2b8d906dcddfada09a395276e141186f4cf8a9627bbf1ef3b612fb83ab` |
| `spec/purl/types/chrome-extension-definition.json` | `types/chrome-extension-definition.json` | `e40f95755b66984bd8d947e1304859c4f3986f98ed2e4f63a63eba48caf1e597` |
| `spec/purl/types/cocoapods-definition.json` | `types/cocoapods-definition.json` | `0a6578c3bb035101ebe3055ba7b5edff169ac3917866d4d46afeaabd740bc51d` |
| `spec/purl/types/composer-definition.json` | `types/composer-definition.json` | `0f3e5450b1688ab7c68d26418f1287863e251faff3c40d7d79632f4ff96656a0` |
| `spec/purl/types/conan-definition.json` | `types/conan-definition.json` | `f32df2052ab6f61c3701578d4ae227cfc0e402fe604b619be5e904cbcaa715e8` |
| `spec/purl/types/conda-definition.json` | `types/conda-definition.json` | `01775daaaa79253f840b52daf5b89754ea76782cdf0dde5b0901e08b98400dc9` |
| `spec/purl/types/cpan-definition.json` | `types/cpan-definition.json` | `cb862b345cfc122677f69da6f8bbc1d697acbbc099b549fce73ccc7269168264` |
| `spec/purl/types/cran-definition.json` | `types/cran-definition.json` | `eafcec94a74cbe8ff9b83cb3244f19387441d7569ea5b5be47fed56592059bf3` |
| `spec/purl/types/deb-definition.json` | `types/deb-definition.json` | `0eb1841949a19309fdbfa25f52f42ebea073bd4cbf8ccf9d0470279a7b696d15` |
| `spec/purl/types/docker-definition.json` | `types/docker-definition.json` | `9adaee3b5ec2fe8a23f63e7c7f9e28262ee771ad82f85f242101e877bb817387` |
| `spec/purl/types/gem-definition.json` | `types/gem-definition.json` | `2fd7f21d76132bccfe8db7e218d75291725628b1aba97e7840c510fd3542c1bb` |
| `spec/purl/types/generic-definition.json` | `types/generic-definition.json` | `ba3cfdb29ec2793966f1c98365e5d70e422b44ef50cd1c468952ff0f19a7a5c6` |
| `spec/purl/types/git-definition.json` | `types/git-definition.json` | `c37eae4004457274d727fd4dce4c195fe0d640e7f2ebf83a5fa728036e9c4a3e` |
| `spec/purl/types/github-definition.json` | `types/github-definition.json` | `72708ea3c3636d2d44ea626e13618c2f739b74fe5e23715e51f111f3401c36ae` |
| `spec/purl/types/golang-definition.json` | `types/golang-definition.json` | `42447a95400f905204ced0517ede501a4d1c5437498232537446469e3696cf06` |
| `spec/purl/types/hackage-definition.json` | `types/hackage-definition.json` | `cdf4754d2421ac38a003553e66f51867b719be1f65f590c1c0b1d5ea5ec0fcf1` |
| `spec/purl/types/hex-definition.json` | `types/hex-definition.json` | `153fb3499e0788b408f09a4fad1b3ec2a2b39ea08ef19bc777218bbdbe726007` |
| `spec/purl/types/huggingface-definition.json` | `types/huggingface-definition.json` | `ce29d4840b1eda705c6adbce6ef6545328901b88693e4b3d04fee1a09b563131` |
| `spec/purl/types/julia-definition.json` | `types/julia-definition.json` | `93b8e73f19040acd97ab71250c1cc37c5712c7dd76d675f2465b8e0e991ff145` |
| `spec/purl/types/luarocks-definition.json` | `types/luarocks-definition.json` | `876cd9d974bf396f29a154a84185b970ce535daaaeb5e95eecac42311593d56d` |
| `spec/purl/types/maven-definition.json` | `types/maven-definition.json` | `dc99b7d08eebf2fa5fc66831e78891be118f3c6012b2db83dd89dcffb85ff7e5` |
| `spec/purl/types/mlflow-definition.json` | `types/mlflow-definition.json` | `ce4d7449702417ddec1dd1ae92d62f8c5cf8b937c97abfc00b681e9dc17c6661` |
| `spec/purl/types/npm-definition.json` | `types/npm-definition.json` | `64c31b4eb4f542fecff9c39dd8a2b9764236b316454c95ff8cfa38d08d0d8e98` |
| `spec/purl/types/nuget-definition.json` | `types/nuget-definition.json` | `6fb9bbd915e2230e381a9d9a0b11b83a9c1cf45404f97dcaa8f4847a96d48f1b` |
| `spec/purl/types/oci-definition.json` | `types/oci-definition.json` | `75bc102a90ed348fce6c1f5d907d61132a430828db5a10445d15951c79c89778` |
| `spec/purl/types/opam-definition.json` | `types/opam-definition.json` | `de9c3257e390d3de285aa0e7b403607720f9a903d9574759ebec306bcdeedf76` |
| `spec/purl/types/otp-definition.json` | `types/otp-definition.json` | `cf367cc3860638f0119ea44b5b37d5c75475b03f3108c76618dd1df6ed250550` |
| `spec/purl/types/pub-definition.json` | `types/pub-definition.json` | `90782999141dac8e67c4299b082acee245af499b41fb65c429e45bcd8e8d50d5` |
| `spec/purl/types/pypi-definition.json` | `types/pypi-definition.json` | `885f90643fd5ad0e1f58650f644f0fd37a62986462202725f0bdfff8a8f36fb9` |
| `spec/purl/types/qpkg-definition.json` | `types/qpkg-definition.json` | `1ac71b010de2a0d5facd52c6fc9b18c79d79c8278ba28532347e27dc72b1702d` |
| `spec/purl/types/rpm-definition.json` | `types/rpm-definition.json` | `deddedde07c0181d4640cdac4ea36928d79531627c7097a540520cdf99f8546a` |
| `spec/purl/types/swid-definition.json` | `types/swid-definition.json` | `14627a8ab84ef21859ca559ce593ff0dd1d0bbb643e25571a977ab500d758ef8` |
| `spec/purl/types/swift-definition.json` | `types/swift-definition.json` | `4d9a6cdec234ac8429eb5fd5033535f14b80d5005f83260645a6200d6a7f393a` |
| `spec/purl/types/vcpkg-definition.json` | `types/vcpkg-definition.json` | `dcc7ed2efe751bd77ece94ab73e150e751169cc48e1b6c6aee73ef23eece2ed2` |
| `spec/purl/types/vscode-extension-definition.json` | `types/vscode-extension-definition.json` | `04452a054245c9ab298e5b022115f653da483c3eb28bb7274bb631ecbd03845b` |
| `spec/purl/types/yocto-definition.json` | `types/yocto-definition.json` | `e18418d544c60ea1a7cfdce6780c65d95dc912f7be41e39a968997cb07f89afe` |
