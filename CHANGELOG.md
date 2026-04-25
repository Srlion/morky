# Changelog

## [0.5.0](https://github.com/Srlion/morky/compare/v0.4.0...v0.5.0) (2026-04-25)


### Features

* add "Volume" tab in app page ([aa01d10](https://github.com/Srlion/morky/commit/aa01d10290dfc8e7c9c2e54702033bc8aace5950))
* add TtlMap implementation for time-based cache expiration ([fff0a05](https://github.com/Srlion/morky/commit/fff0a05fe584821d57519ff8f499750a75baaf8e))


### Bug Fixes

* cleanup marker only when successful ([92b7168](https://github.com/Srlion/morky/commit/92b7168b9f488776357252e4ec205d324a1cd442))
* fix install script to replace existing buildkit container ([5fd093d](https://github.com/Srlion/morky/commit/5fd093df5342b19662d18e59fb18d72f60954fb3))
* replace global state with TtlMap ([0f60653](https://github.com/Srlion/morky/commit/0f60653f4084787a8d7989fd70061cf61b3ca8fe))

## [0.4.0](https://github.com/Srlion/morky/compare/v0.3.1...v0.4.0) (2026-04-24)


### Features

* add mimalloc as global allocator ([a693f3b](https://github.com/Srlion/morky/commit/a693f3bf875a5e25d48770bbf4ae38baa09d1403))


### Bug Fixes

* try to lower memory usage ([47b857d](https://github.com/Srlion/morky/commit/47b857deb6e1a67e4f250a4224827881e77cbcb9))
* try to lower memory usage ([2fc930f](https://github.com/Srlion/morky/commit/2fc930ffeb8270615269b17fb6c9cfac19af6a1d))

## [0.3.1](https://github.com/Srlion/morky/compare/v0.3.0...v0.3.1) (2026-04-24)


### Bug Fixes

* fix first time install ([d81890a](https://github.com/Srlion/morky/commit/d81890ae9eea774f50245a90b372d8ae254ba44b))

## [0.3.0](https://github.com/Srlion/morky/compare/v0.2.0...v0.3.0) (2026-04-24)


### Features

* cache settings in memory and make get() sync ([9462c6c](https://github.com/Srlion/morky/commit/9462c6c8004ab2bd435d973f971c30375786c1d8))


### Bug Fixes

* simplify podman installation command ([379c351](https://github.com/Srlion/morky/commit/379c351d979001377db575b9131531500f8aa4e4))
* some quick cleanup ([552f442](https://github.com/Srlion/morky/commit/552f442c6a1282a1c9fe63ae7a0a863458f26ac1))

## [0.2.0](https://github.com/Srlion/morky/compare/v0.1.0...v0.2.0) (2026-04-24)


### Features

* support restoring from backup via --restore flag ([0ad4cb9](https://github.com/Srlion/morky/commit/0ad4cb9d24fdafbbd5c7f09bc92a99026c4836a8))


### Bug Fixes

* add settings tabs and password change functionality ([35c6e40](https://github.com/Srlion/morky/commit/35c6e40f118ddf6bf15c30e83b55bb9f0295501a))
* add version.txt to backup tar archive ([bac7417](https://github.com/Srlion/morky/commit/bac74178a9ff12c352517961dc7506143580b7a2))
* get rid of old_build.rs ([dc25d16](https://github.com/Srlion/morky/commit/dc25d16140cff766573e840ad4f569762aac182e))
* remove frontend watcher ([9a4ffcb](https://github.com/Srlion/morky/commit/9a4ffcb2e1b982650ec2db41f5cad1fde5b28dba))
* update monitoring to use VecDeque for efficient history management ([dd0b1c1](https://github.com/Srlion/morky/commit/dd0b1c1cb61a8983cc235080374ff1da6fd011cd))
* update SQLite pragmas for max durability ([95699fb](https://github.com/Srlion/morky/commit/95699fb620dfa109d64ac7761553704715f3d808))


### Miscellaneous Chores

* release 0.2.0 ([e01e8da](https://github.com/Srlion/morky/commit/e01e8da1921a502231485d844181fd209fb95527))
* set version to 0.1.0 ([8195086](https://github.com/Srlion/morky/commit/8195086e18718c002ff00ec6fc50e2f0ec17aa9d))
* set version to 0.1.0 oops ([39a9076](https://github.com/Srlion/morky/commit/39a90767a2b0dd8eb9e91d47c4aed51db160590f))

## 0.1.0 (2026-04-24)


### Features

* support restoring from backup via --restore flag ([0ad4cb9](https://github.com/Srlion/morky/commit/0ad4cb9d24fdafbbd5c7f09bc92a99026c4836a8))


### Bug Fixes

* add settings tabs and password change functionality ([35c6e40](https://github.com/Srlion/morky/commit/35c6e40f118ddf6bf15c30e83b55bb9f0295501a))
* add version.txt to backup tar archive ([bac7417](https://github.com/Srlion/morky/commit/bac74178a9ff12c352517961dc7506143580b7a2))
* get rid of old_build.rs ([dc25d16](https://github.com/Srlion/morky/commit/dc25d16140cff766573e840ad4f569762aac182e))
* remove frontend watcher ([9a4ffcb](https://github.com/Srlion/morky/commit/9a4ffcb2e1b982650ec2db41f5cad1fde5b28dba))
* update monitoring to use VecDeque for efficient history management ([dd0b1c1](https://github.com/Srlion/morky/commit/dd0b1c1cb61a8983cc235080374ff1da6fd011cd))
* update SQLite pragmas for max durability ([95699fb](https://github.com/Srlion/morky/commit/95699fb620dfa109d64ac7761553704715f3d808))


### Miscellaneous Chores

* set version to 0.1.0 ([8195086](https://github.com/Srlion/morky/commit/8195086e18718c002ff00ec6fc50e2f0ec17aa9d))
* set version to 0.1.0 oops ([39a9076](https://github.com/Srlion/morky/commit/39a90767a2b0dd8eb9e91d47c4aed51db160590f))

## 0.1.0 (2026-04-21)


### Bug Fixes

* add token to release-please workflow ([b9062b5](https://github.com/Srlion/morky/commit/b9062b56777e4c99abd4511b1da618cbce3f718f))
* has to be this ([d9791f1](https://github.com/Srlion/morky/commit/d9791f182abf369c1a834b9dfdcda36506c12e7b))
* try this ([4208d92](https://github.com/Srlion/morky/commit/4208d920741099ac2c191c107c7d9160a77c3ede))
* try this one maybe ([1e449dc](https://github.com/Srlion/morky/commit/1e449dceefd4d4d432e59bacaa42cafea9374703))


### Miscellaneous Chores

* initial release ([5050e79](https://github.com/Srlion/morky/commit/5050e79a8cad65996a83036e8b4f544d9bfe6c62))
