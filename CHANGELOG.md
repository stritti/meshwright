# Changelog

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
