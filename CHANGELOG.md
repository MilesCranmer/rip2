# Changelog

## [0.9.3](https://github.com/MilesCranmer/rip2/compare/v0.9.2...v0.9.3) (2025-01-14)


### Features

* add `-f,--force` flag ([24c460a](https://github.com/MilesCranmer/rip2/commit/24c460a8ed5b74f0de8fb524e653b6c7da2bb36b))


### Bug Fixes

* canonicalize graveyard path in check ([87322f9](https://github.com/MilesCranmer/rip2/commit/87322f9f7d4ddcfbca31b247c8324c04c7d8df38))

## [0.9.2](https://github.com/MilesCranmer/rip2/compare/v0.9.1...v0.9.2) (2024-12-24)


### Features

* include header validation ([e36bcc6](https://github.com/MilesCranmer/rip2/commit/e36bcc6e3de4a4ef4bd13f02ff7ad4ae74946c7e))
* nicer error message for old `rip` format ([98e6b83](https://github.com/MilesCranmer/rip2/commit/98e6b83c0ed37abef1f0ddbb2d962cee41e702ba))

## [0.9.1](https://github.com/MilesCranmer/rip2/compare/v0.9.0...v0.9.1) (2024-12-22)


### Features

* generate auto-completion in flake build ([#65](https://github.com/MilesCranmer/rip2/issues/65)) ([0b6241a](https://github.com/MilesCranmer/rip2/commit/0b6241abda2b2a2da8cdef98f681bc1467085298))

## [0.9.0](https://github.com/MilesCranmer/rip2/compare/v0.8.3...v0.9.0) (2024-10-22)


### ⚠ BREAKING CHANGES

* const generic parameter for Record to disable file lock
* create file locking via fs4

### Features

* const generic parameter for Record to disable file lock ([c61a7d8](https://github.com/MilesCranmer/rip2/commit/c61a7d85e2d3b61e6ec707c9a117e5fb79a14ea2))
* create file locking via fs4 ([8e0bd96](https://github.com/MilesCranmer/rip2/commit/8e0bd962cb707dee3e7a4854b5406fe53a21ec73))


### Bug Fixes

* be more careful about locking ([88cb8a0](https://github.com/MilesCranmer/rip2/commit/88cb8a01596e3ebad31fa995ce3897fad408a8a6))
* disable fs4 on windows ([fbf8ed5](https://github.com/MilesCranmer/rip2/commit/fbf8ed57feed7ab9ebda20db7cbb9378cd2bfff3))

## [0.8.3](https://github.com/MilesCranmer/rip2/compare/v0.8.2...v0.8.3) (2024-10-22)


### Bug Fixes

* [#54](https://github.com/MilesCranmer/rip2/issues/54) with missed metadata ([a44e992](https://github.com/MilesCranmer/rip2/commit/a44e992b4ba8d99145b8180fef2d29fefd339926))
* mistaken deletion of header when deleting lines ([bbebe2d](https://github.com/MilesCranmer/rip2/commit/bbebe2d8590b61bbf684dcc751ec8ca3590fe0be))
* potential issue with missing header ([0a7e2bc](https://github.com/MilesCranmer/rip2/commit/0a7e2bcf01690ffd770261e88e634b88536d3793))

## [0.8.2](https://github.com/MilesCranmer/rip2/compare/v0.8.1...v0.8.2) (2024-08-12)


### Features

* name available completions in help message ([4ddc3e7](https://github.com/MilesCranmer/rip2/commit/4ddc3e70e2c3d1a9530e78a38938ce6558242181))

## [0.8.1](https://github.com/MilesCranmer/rip2/compare/v0.8.0...v0.8.1) (2024-07-01)


### Miscellaneous Chores

* trigger release ([e46d261](https://github.com/MilesCranmer/rip2/commit/e46d2613e539566760521905d85194690d6efba2))

## [0.8.0](https://github.com/MilesCranmer/rip2/compare/v0.7.0...v0.8.0) (2024-05-07)


### ⚠ BREAKING CHANGES

* output column headers during seance

### Features

* display deletion time during seance ([19dbe0a](https://github.com/MilesCranmer/rip2/commit/19dbe0a3bb61b62727cc5a4ab98913feb3c01512))
* output column headers during seance ([d29bdd5](https://github.com/MilesCranmer/rip2/commit/d29bdd51e93e8cccacbfc12e5e99608cf1c07314))

## [0.7.0](https://github.com/MilesCranmer/rip2/compare/v0.6.1...v0.7.0) (2024-04-22)


### ⚠ BREAKING CHANGES

* store header in record

### Features

* store header in record ([452c476](https://github.com/MilesCranmer/rip2/commit/452c476c83a4587dae9b447c7f4a3ebb2c55cf0d))

## [0.6.1](https://github.com/MilesCranmer/rip2/compare/v0.6.0...v0.6.1) (2024-04-22)


### Bug Fixes

* avoid printing newline for graveyard subcommand ([bc8315d](https://github.com/MilesCranmer/rip2/commit/bc8315d1f096a969999f1c1f1d8db8295e992027))

## [0.6.0](https://github.com/MilesCranmer/rip2/compare/v0.5.2...v0.6.0) (2024-04-22)


### ⚠ BREAKING CHANGES

* switch display to binary prefix

### Features

* `fs_extra` to get dir sizes ([ceda4a9](https://github.com/MilesCranmer/rip2/commit/ceda4a974a68d1ef48cd58322e49118f507ba076))
* sort entries in inspection mode ([7686ea3](https://github.com/MilesCranmer/rip2/commit/7686ea362f631ed8a877963c56e1eccbb24172c6))
* switch display to binary prefix ([8fd45f1](https://github.com/MilesCranmer/rip2/commit/8fd45f1e0eb95a217756363a0e3dfda99db7dd21))


### Reverts

* feat!: switch from walkdir to jwalk for parallelism ([efaa396](https://github.com/MilesCranmer/rip2/commit/efaa396054c0e0f6b12dabb01147a4481db298f8))

## [0.5.2](https://github.com/MilesCranmer/rip2/compare/v0.5.1...v0.5.2) (2024-04-15)


### Features

* better error when no record found ([2dcc4af](https://github.com/MilesCranmer/rip2/commit/2dcc4af2babe4cd2df3dcea2b75169e603fabeba))

## [0.5.1](https://github.com/MilesCranmer/rip2/compare/v0.5.0...v0.5.1) (2024-04-15)


### Features

* add seance option to graveyard subcommand ([ad85c0f](https://github.com/MilesCranmer/rip2/commit/ad85c0fd517f476bf4d141a8a1c30e173c43152d))
* add subcommand to get graveyard path ([448caf7](https://github.com/MilesCranmer/rip2/commit/448caf7b6c6c86bd8fa02783d4b7e70064725d11))
* use colors in help menus ([261e69d](https://github.com/MilesCranmer/rip2/commit/261e69d7d3671b5b131ac8458eb10c462098ea34))

## [0.5.0](https://github.com/MilesCranmer/rip2/compare/v0.4.0...v0.5.0) (2024-04-15)


### ⚠ BREAKING CHANGES

* use `env::temp_dir` for graveyard path ([#22](https://github.com/MilesCranmer/rip2/issues/22))

### Features

* use `env::temp_dir` for graveyard path ([#22](https://github.com/MilesCranmer/rip2/issues/22)) ([e3eebff](https://github.com/MilesCranmer/rip2/commit/e3eebffc941aa8540b73214d3e4bf5960a4cd254))

## [0.4.0](https://github.com/MilesCranmer/rip2/compare/v0.3.0...v0.4.0) (2024-04-15)


### ⚠ BREAKING CHANGES

* do not record permanent deletions in record
* use dunce canonicalization for windows compat

### Features

* add preliminary windows support ([51bcdf3](https://github.com/MilesCranmer/rip2/commit/51bcdf3e0143858b0e17ea1a31fbaa6b3a90683c))
* do not record permanent deletions in record ([a77e027](https://github.com/MilesCranmer/rip2/commit/a77e027c383af922fec1eeda4eb855b5f82d3bbf))
* more readable logging for windows ([f494d9e](https://github.com/MilesCranmer/rip2/commit/f494d9e3b45210b74ab55a9efc6792e321912a43))
* quit prompt read if given invalid char ([51b0dcf](https://github.com/MilesCranmer/rip2/commit/51b0dcfc4fddca4e799895053d2b68f913ca6371))


### Bug Fixes

* correct behavior for \n stdin ([5c60870](https://github.com/MilesCranmer/rip2/commit/5c608704a16ff36d143a665d2789da3bc67a692f))
* correct behavior for non-input stdin ([b4035a4](https://github.com/MilesCranmer/rip2/commit/b4035a4c240a839cfe3c25607fef07edf2463912))
* correct symlink to symlink_file on windows ([d1ca9ca](https://github.com/MilesCranmer/rip2/commit/d1ca9ca27e35a9dd45c40d31785d76d18820a675))
* seance paths on windows ([9c0d2d5](https://github.com/MilesCranmer/rip2/commit/9c0d2d516fa4146dcb2971a6482b75dfd7f23d59))
* use dunce canonicalization for windows compat ([0d3dc2a](https://github.com/MilesCranmer/rip2/commit/0d3dc2abe6086f7c8460c7552a9cc610ed07bb49))
* workaround for device paths on windows ([6624147](https://github.com/MilesCranmer/rip2/commit/66241479e0f95793b167dc186175e533e4e351c0))

## [0.3.0](https://github.com/MilesCranmer/rip2/compare/v0.2.1...v0.3.0) (2024-04-14)


### ⚠ BREAKING CHANGES

* use subcommands for shell completions

### Features

* use subcommands for shell completions ([adbb270](https://github.com/MilesCranmer/rip2/commit/adbb270190a80a33515b091d50f8c0455029c9c6))


### Bug Fixes

* correct output of shell completions ([67ee0df](https://github.com/MilesCranmer/rip2/commit/67ee0dfb44ae518c68113c857aea093bbf2de62b))

## [0.2.1](https://github.com/MilesCranmer/rip2/compare/v0.2.0...v0.2.1) (2024-04-11)


### Bug Fixes

* flush stream even if not stdout ([09504c8](https://github.com/MilesCranmer/rip2/commit/09504c8b8d16d07aa973ace093b80485a87ee32e))

## [0.2.0](https://github.com/MilesCranmer/rip2/compare/v0.1.0...v0.2.0) (2024-04-09)


### Features

* test feat ([11656a2](https://github.com/MilesCranmer/rip2/commit/11656a2c3216fed0dc6b3a4566641d8c571bf107))
