# Changelog

## [0.1.0-beta.2](https://github.com/stritti/meshwright/compare/v0.1.0-beta.1...v0.1.0-beta.2) (2026-11-22)


### Features

* name parameters as mesh2cad does and keep the part in place ([#9](https://github.com/stritti/meshwright/issues/9)) ([da49758](https://github.com/stritti/meshwright/commit/da49758e90e2e7c9f38cd00923b02fbd959a40d4))
* **plate:** recognise blind holes ([#14](https://github.com/stritti/meshwright/issues/14)) ([a6c067c](https://github.com/stritti/meshwright/commit/a6c067c3e8a3a55f15b9e9062e874b7e43f76ad2))
* **plate:** recognise polygonal cut-outs ([#21](https://github.com/stritti/meshwright/issues/21)) ([ee4bcf6](https://github.com/stritti/meshwright/commit/ee4bcf609203c6dd0fe061069c1ed3f42d930499))
* **plate:** recognise rectangular pads and pockets ([#22](https://github.com/stritti/meshwright/issues/22)) ([ac47e78](https://github.com/stritti/meshwright/commit/ac47e787a20344b6b6c5e27fb326fe2b14f337ad))
* **plate:** recognise round bosses on the top face ([#20](https://github.com/stritti/meshwright/issues/20)) ([f77c985](https://github.com/stritti/meshwright/commit/f77c9850b302991219d06e67ec7eaf5f41a51307))
* **plate:** recognise straight slots ([#15](https://github.com/stritti/meshwright/issues/15)) ([2bc1a2b](https://github.com/stritti/meshwright/commit/2bc1a2bb8e725749963388f62fe86033ecc9939c))
* **recognize:** counterbored and countersunk holes ([#13](https://github.com/stritti/meshwright/issues/13)) ([aa80639](https://github.com/stritti/meshwright/commit/aa80639752a961193fe60db18a53c95b57422be7))
* **recognize:** hole rows, grids and bolt circles as SolidCraft patterns ([#11](https://github.com/stritti/meshwright/issues/11)) ([3c4a64d](https://github.com/stritti/meshwright/commit/3c4a64d0a9fd9a034efe510e0d687d1ca7ee84e6))


### Bug Fixes

* **import:** preserve placement and handle STL input limits ([#32](https://github.com/stritti/meshwright/issues/32)) ([9769821](https://github.com/stritti/meshwright/commit/9769821494416353a5e5596efb91426c6b10ea8d))
* **recognize:** recognise STL files that SolidCraft itself exports ([#12](https://github.com/stritti/meshwright/issues/12)) ([1839760](https://github.com/stritti/meshwright/commit/18397609897781bcdd527dc87731a608b3da27b0))


### Documentation

* explain how to use the plugin in SolidCraft ([#6](https://github.com/stritti/meshwright/issues/6)) ([2147de1](https://github.com/stritti/meshwright/commit/2147de1bc9db6556a5f263d369c628618d7f36ea))
* publish a VitePress site on GitHub Pages ([#10](https://github.com/stritti/meshwright/issues/10)) ([e8a6486](https://github.com/stritti/meshwright/commit/e8a64864f5147c13dc425c249a02d8d6a6f0f780))

## 0.1.0-beta.1 (2026-11-21)


### Features

* project skeleton for the meshwright SolidCraft importer ([66706d6](https://github.com/stritti/meshwright/commit/66706d694a40899bc86997f3fdc193fa72676e23))
* **recognize:** recognise plates with any outline of straight sides ([5e248b4](https://github.com/stritti/meshwright/commit/5e248b45ef2a1034b19e6d5bfae7673d443065d4))
* **recognize:** recognise plates with holes, rounded corners and any straight-sided outline ([9ac04bd](https://github.com/stritti/meshwright/commit/9ac04bd00a83ebc97b8e5c22090fbb06822db883))
* **recognize:** recognise plates with through holes and rounded corners ([24eae8e](https://github.com/stritti/meshwright/commit/24eae8e5f368dd31800bdc567ae169666920635a))


### Bug Fixes

* **recognize:** fit rounded corners robustly in single-precision STL ([533eb14](https://github.com/stritti/meshwright/commit/533eb14d90fa5969e2b54ec2157cc88997e3c737))
* **recognize:** refuse D-shaped cut-outs, fillet before holes, linear outline scan ([f96ce65](https://github.com/stritti/meshwright/commit/f96ce6569224bf04d0119db0afd9ab59dd0e09a2))
* stream large STL files, reject broken ones, own token for release-please ([#5](https://github.com/stritti/meshwright/issues/5)) ([8313750](https://github.com/stritti/meshwright/commit/8313750630eeb66a12b0a54fc4cfbf52b11f2b1c))
