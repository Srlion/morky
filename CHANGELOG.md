# Changelog

## [0.8.12](https://github.com/Srlion/morky/compare/v0.8.11...v0.8.12) (2026-07-22)


### Bug Fixes

* add the mnissing SUPERSEDED status to DeployStatus and update deployment status checks ([6945089](https://github.com/Srlion/morky/commit/6945089822d496e05548cef58dceebbda0509c1b))

## [0.8.11](https://github.com/Srlion/morky/compare/v0.8.10...v0.8.11) (2026-07-22)


### Bug Fixes

* trigger build ([beafc92](https://github.com/Srlion/morky/commit/beafc920d0a3f3a167a84885bc587359392fe412))

## [0.8.10](https://github.com/Srlion/morky/compare/v0.8.9...v0.8.10) (2026-07-22)


### Bug Fixes

* try to trace the memory leak ([50a1ee0](https://github.com/Srlion/morky/commit/50a1ee05bf78a0eb17393fbde61a5d425b613455))

## [0.8.9](https://github.com/Srlion/morky/compare/v0.8.8...v0.8.9) (2026-07-22)


### Bug Fixes

* update dependencies in package.json ([3069f43](https://github.com/Srlion/morky/commit/3069f436b0f6577ab37df519c6b01c91f6ab666e))

## [0.8.8](https://github.com/Srlion/morky/compare/v0.8.7...v0.8.8) (2026-07-22)


### Bug Fixes

* add delete_for_user_except method to Session model and integrate it into password change ([ab2c087](https://github.com/Srlion/morky/commit/ab2c08797fff1fcd2f406ba1323ff0dec4c076fd))
* add missing Superseded status to DeployStatus enum ([3ee4a9d](https://github.com/Srlion/morky/commit/3ee4a9db2c77cd336fc01f967490fc12efe16af2))
* add unique index for port in apps table to prevent duplicates ([214ac7d](https://github.com/Srlion/morky/commit/214ac7da82c24c0bfbbd7ab4ec173ea6e62725cf))
* avoid port collision with panel value in next_available_port function ([6fbfd31](https://github.com/Srlion/morky/commit/6fbfd3121657dce73f02d84bc13ec9d92d053620))
* change error response status from 500 to 400 for invalid JSON requests ([c965aae](https://github.com/Srlion/morky/commit/c965aae06184d1b8c750fbc54e364706e19824ad))
* cleanup volume.rs ([3b1bd3e](https://github.com/Srlion/morky/commit/3b1bd3ed714df5b9489532ba2161e60270c9c4a5))
* deploy status model (superseded/cancelled) + rollback check ([7c5ef04](https://github.com/Srlion/morky/commit/7c5ef043ba90d2763677d6742338052a9e7cbf22))
* enable kill_on_drop for command execution for safety ([78c9189](https://github.com/Srlion/morky/commit/78c91892dcd893d9066a73099c0768bd866b20e9))
* enhance app deletion process by stopping containers, removing images, and cleaning up volumes ([da35979](https://github.com/Srlion/morky/commit/da359790ec8a49656066f5f6eefee038b7c2de1e))
* enhance HTTP client configuration with connect and request timeouts ([60d4755](https://github.com/Srlion/morky/commit/60d4755d24fa86b09803ca0af077a4578daed888))
* fix set_current_deployment updating logic ([8c70545](https://github.com/Srlion/morky/commit/8c705450896661fc8ef0f3a8612001608a915331))
* handle build cancellation correctly ([cfbe46e](https://github.com/Srlion/morky/commit/cfbe46ed41b098e1f3f4aa9b6f0b55ec0a4c4a7e))
* implement periodic cleanup of expired sessions in main function ([28ed501](https://github.com/Srlion/morky/commit/28ed501625d356fec97fe3a79b31b9940db249ba))
* improve error handling for buildkit initialization ([788f501](https://github.com/Srlion/morky/commit/788f501c80b9fc44fb6da346785b21a96ca34b15))
* optimize image existence check in list_deployments function ([f8e6fbf](https://github.com/Srlion/morky/commit/f8e6fbffaf8a31068ea35e077108a45537306530))
* redact sensitive information in command logging ([0dc3f8b](https://github.com/Srlion/morky/commit/0dc3f8bd292b0cd543b62ab54a95e97240969185))
* reduce sleep duration in poll loop from 5 seconds to 1 second ([88a90b7](https://github.com/Srlion/morky/commit/88a90b7a004338b65c967dbfa3b34f43e24f4da2))
* refactor deployment retrieval logic to improve error handling and reduce code duplication ([e920f28](https://github.com/Srlion/morky/commit/e920f28ca881832a37beea07cb10ce8ae42a5587))
* refactor reconcile function to build routes directly from database query results ([ab47224](https://github.com/Srlion/morky/commit/ab47224c53bb26aff0e0bae0d64ad2f6f5592e50))
* reorder snapshot retrieval in sse_handler ([0d2b0cf](https://github.com/Srlion/morky/commit/0d2b0cf865fb7d3ea56e245e3a1ec4785520ef2e))
* replace direct file write with write_secret_file in auto_setup and constants ([0a7eb3a](https://github.com/Srlion/morky/commit/0a7eb3a9f07d21e6f697dde4bc4691c82d7cbf09))
* secrets_hash determinism ([3660b97](https://github.com/Srlion/morky/commit/3660b97c6ff232fa01a176d36e376d5488908389))
* update UpdateBody struct to support nested option for panel_domain and improve domain validation logic ([f4f2353](https://github.com/Srlion/morky/commit/f4f23539449c37a58cee38abb02f53b9150d67b8))
* update volume cleanup logic to remove dangling volumes and skip specific ones ([3df632b](https://github.com/Srlion/morky/commit/3df632b4311f6e12dc2c8735203291acd32645b8))

## [0.8.7](https://github.com/Srlion/morky/compare/v0.8.6...v0.8.7) (2026-07-22)


### Bug Fixes

* actually fix deploying ([4d7eca9](https://github.com/Srlion/morky/commit/4d7eca90a22981afa3b171d6d60b1ceb2845beaa))

## [0.8.6](https://github.com/Srlion/morky/compare/v0.8.5...v0.8.6) (2026-07-22)


### Bug Fixes

* bump version correctly ([94d21f8](https://github.com/Srlion/morky/commit/94d21f8fccf50608bbded599a169bff487b27800))
* fix backups not working correctly ([af6325e](https://github.com/Srlion/morky/commit/af6325e086b90e611905cbdec85d774dc06b20c6))
* fix deploying failing at the end ([82b599f](https://github.com/Srlion/morky/commit/82b599fc8d4b4ca44e21ca9b713a97c6306ff112))
* fix release-please failing ([56a8717](https://github.com/Srlion/morky/commit/56a87173366d27796f00244675a8cd252963e768))

## [0.8.5](https://github.com/Srlion/morky/compare/v0.8.4...v0.8.5) (2026-07-22)


### Bug Fixes

* try to get rid of small memory leaks ([1f1c717](https://github.com/Srlion/morky/commit/1f1c717b7343d07d08726844939b1cb4d5060242))
* update maw dependency version to latest ([a1d32a2](https://github.com/Srlion/morky/commit/a1d32a22bb15ef54310f75f8c4674860576fb730))

## [0.8.4](https://github.com/Srlion/morky/compare/v0.8.3...v0.8.4) (2026-07-22)


### Bug Fixes

* add CONTAINERS_POLICY_JSON environment variable to service configuration ([2643570](https://github.com/Srlion/morky/commit/264357056bbf8a965242c8183350fd346d9704e3))

## [0.8.3](https://github.com/Srlion/morky/compare/v0.8.2...v0.8.3) (2026-07-22)


### Bug Fixes

* publish as prerelease till "release" job finishes ([391a243](https://github.com/Srlion/morky/commit/391a243e8265a6ddfedc0df2e53223cdf0c00274))

## [0.8.2](https://github.com/Srlion/morky/compare/v0.8.1...v0.8.2) (2026-07-22)


### Bug Fixes

* make buildkit address configurable via environment variable ([8ae58bd](https://github.com/Srlion/morky/commit/8ae58bd50bf2da0b19ca6b38f320227bcc521374))
* update Deno and Debian base images, and switch to crun for container runtime ([be73bca](https://github.com/Srlion/morky/commit/be73bca731d85a623eebf6ff305e76457923974f))
* update dependencies in Cargo.toml ([a5253ce](https://github.com/Srlion/morky/commit/a5253cefaa162716b0025374f1c48921543ba5a8))
* use regex-lite over regex crate ([5c5180a](https://github.com/Srlion/morky/commit/5c5180aa747b75e8a18990403593051fe09856ec))

## [0.8.1](https://github.com/Srlion/morky/compare/v0.8.0...v0.8.1) (2026-05-01)


### Bug Fixes

* update dependencies to latest ([304f95e](https://github.com/Srlion/morky/commit/304f95eb8827083897862f9b4f318cd32b36d7a7))

## [0.8.0](https://github.com/Srlion/morky/compare/v0.7.0...v0.8.0) (2026-04-26)


### Features

* lots of stuff idc ([1c2eeec](https://github.com/Srlion/morky/commit/1c2eeeca815cdf6ff3fdf8e3673d42ab9335b97a))


### Bug Fixes

* fix install script to install quadlets ([867da17](https://github.com/Srlion/morky/commit/867da1782afd5110ba31b6f02b2087350632a5ad))
* fix logs page in app not showing the newest ([44f924d](https://github.com/Srlion/morky/commit/44f924d40b5a6758bce13a7c9cd8d33d71146ec7))
* fix shutdown signal to be received correctly ([39680bf](https://github.com/Srlion/morky/commit/39680bf574a01b60c863daff43dcf69a9ffb958e))
* remove buildkit container creation from install script ([50d76e2](https://github.com/Srlion/morky/commit/50d76e215a5ee2ee5e3ef174f9ea6d5dd7658617))

## [0.7.0](https://github.com/Srlion/morky/compare/v0.6.3...v0.7.0) (2026-04-25)


### Features

* limit buildkit cpu usage ([f7b21b1](https://github.com/Srlion/morky/commit/f7b21b13b78cf18bcd132c2eabe3c229c08a087c))


### Bug Fixes

* fix env vars dialog looking terrible ([36869fb](https://github.com/Srlion/morky/commit/36869fb191c6f5a918880d65af699638a53649e1))

## [0.6.3](https://github.com/Srlion/morky/compare/v0.6.2...v0.6.3) (2026-04-25)


### Bug Fixes

* add ansi-to-html dependency and render logs with ANSI color codes ([2d4eb48](https://github.com/Srlion/morky/commit/2d4eb48a57a2f3e5120e51c3cc5eb217774b58ba))

## [0.6.2](https://github.com/Srlion/morky/compare/v0.6.1...v0.6.2) (2026-04-25)


### Bug Fixes

* fix volume path not saving for docker build type ([e3ad93f](https://github.com/Srlion/morky/commit/e3ad93fccaaefd826404635961a6b2c435057785))

## [0.6.1](https://github.com/Srlion/morky/compare/v0.6.0...v0.6.1) (2026-04-25)


### Bug Fixes

* remove conditional volume path input for Railpack builds ([3fd0910](https://github.com/Srlion/morky/commit/3fd09107b660fdc97375724aa45ea08d63bd9cac))

## [0.6.0](https://github.com/Srlion/morky/compare/v0.5.1...v0.6.0) (2026-04-25)


### Features

* add drag and drop file upload and folder creation ([6771e83](https://github.com/Srlion/morky/commit/6771e8329675b4152a2d9ffdddf49f331a282998))


### Bug Fixes

* make drag n drop work inside app volume too ([f931eb4](https://github.com/Srlion/morky/commit/f931eb47a0c89cdb3b45cbe942a84056c7bb58e5))

## [0.5.1](https://github.com/Srlion/morky/compare/v0.5.0...v0.5.1) (2026-04-25)


### Bug Fixes

* fix volumes not working correctly ([1bf80ea](https://github.com/Srlion/morky/commit/1bf80eada860989558158b84377ef83601d330df))

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
