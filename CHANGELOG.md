# Changelog

## [0.6.1](https://github.com/MilesCranmer/rip2/compare/v0.6.0...v0.6.1) (2024-04-22)


### Bug Fixes

* avoid printing newline for graveyard subcommand ([f475bae](https://github.com/MilesCranmer/rip2/commit/f475bae3c7852b32ceb8b431ea6ad5cd22866dd9))

## [0.6.0](https://github.com/MilesCranmer/rip2/compare/v0.5.2...v0.6.0) (2024-04-22)


### ⚠ BREAKING CHANGES

* switch from walkdir to jwalk for parallelism
* switch display to binary prefix

### Features

* `fs_extra` to get dir sizes ([ceda4a9](https://github.com/MilesCranmer/rip2/commit/ceda4a974a68d1ef48cd58322e49118f507ba076))
* sort entries in inspection mode ([7686ea3](https://github.com/MilesCranmer/rip2/commit/7686ea362f631ed8a877963c56e1eccbb24172c6))
* switch display to binary prefix ([8fd45f1](https://github.com/MilesCranmer/rip2/commit/8fd45f1e0eb95a217756363a0e3dfda99db7dd21))
* switch from walkdir to jwalk for parallelism ([c1d7d09](https://github.com/MilesCranmer/rip2/commit/c1d7d09df9131cc52b19905dc8b8c0718d5a36c9))


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
