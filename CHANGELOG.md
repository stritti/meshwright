# Changelog

## [0.1.0-beta.6](https://github.com/stritti/revmesh/compare/v0.1.0-beta.5...v0.1.0-beta.6) (2026-11-25)


### Features

* check for a newer release on every import ([#63](https://github.com/stritti/revmesh/issues/63)) ([a92cbe9](https://github.com/stritti/revmesh/commit/a92cbe9cd350843657a504d36382b9214bdbf79a))
* **design:** angle the longest slanted side of an outline or cut-out sketch (roadmap 5.1) ([#72](https://github.com/stritti/revmesh/issues/72)) ([5b1e1b0](https://github.com/stritti/revmesh/commit/5b1e1b0b779e1f6f7550bbd33d824f0c430a5688))
* **design:** dimension every side of an outline and cut-out sketch ([#67](https://github.com/stritti/revmesh/issues/67)) ([75afacf](https://github.com/stritti/revmesh/commit/75afacfd73e6f3128c052b1f089f8eb5ce59b872))
* **plate:** recognise arc sides in outlines as sketch arcs ([#68](https://github.com/stritti/revmesh/issues/68)) ([3a9c9da](https://github.com/stritti/revmesh/commit/3a9c9da8495bfa5faf21627ef7153c31bdfbc4bf))
* **plate:** recognise pockets of any straight-sided outline ([#69](https://github.com/stritti/revmesh/issues/69)) ([c262639](https://github.com/stritti/revmesh/commit/c262639f5270920f9f7c00497386c613a9d3921d))
* **plate:** recognise steps down from the top face (along Z) ([#71](https://github.com/stritti/revmesh/issues/71)) ([e37fcf1](https://github.com/stritti/revmesh/commit/e37fcf1a49bc0e5a1ae13ec0654a33caa27667f1))
* **plugin:** give the stl importer its own icon in SolidCraft's Plugins menu ([#75](https://github.com/stritti/revmesh/issues/75)) ([f3f065b](https://github.com/stritti/revmesh/commit/f3f065b180527939e72c85b640db9d56b294c987))
* **report:** per-face surface deviation in the import report (roadmap 5.4) ([#74](https://github.com/stritti/revmesh/issues/74)) ([8d26b4c](https://github.com/stritti/revmesh/commit/8d26b4c595b6c52ca9ad1825bdfc3322363ae8da))


### Documentation

* add example STL parts and real Printables files with a gallery ([#61](https://github.com/stritti/revmesh/issues/61)) ([1a1d98b](https://github.com/stritti/revmesh/commit/1a1d98ba01ebf346bf0b0278ef556e50a413241d))
* add revmesh-recognition-knowhow agent skill ([#64](https://github.com/stritti/revmesh/issues/64)) ([203b1fc](https://github.com/stritti/revmesh/commit/203b1fc94b78ffd89476b4a9433ac46a26ed47f0))
* add roadmap items 6 and 7 from the code review against the references ([#66](https://github.com/stritti/revmesh/issues/66)) ([770957f](https://github.com/stritti/revmesh/commit/770957f01cc51ad6d58182a20b36cf2057d59bcb))

## [0.1.0-beta.5](https://github.com/stritti/revmesh/compare/v0.1.0-beta.4...v0.1.0-beta.5) (2026-11-24)


### Features

* **combined:** recognise openings through the outlines ([#58](https://github.com/stritti/revmesh/issues/58)) ([17e6bc2](https://github.com/stritti/revmesh/commit/17e6bc24cb501e6d6dc9bc491450f346db527909))
* **combined:** recognise round holes in combined extrusions ([#60](https://github.com/stritti/revmesh/issues/60)) ([da8d7cd](https://github.com/stritti/revmesh/commit/da8d7cd655430be29415072fee3e5afa1ec19ec9))
* **combined:** recognise straight blind cuts and pockets ([#59](https://github.com/stritti/revmesh/issues/59)) ([c79e558](https://github.com/stritti/revmesh/commit/c79e5583df40d300694b5aa6afec47800536039e))
* **plate:** recognise drafted bosses and chamfered boss tops ([#52](https://github.com/stritti/revmesh/issues/52)) ([b50ea33](https://github.com/stritti/revmesh/commit/b50ea33ca54d70a509aac15130050fda57bff897))
* **plate:** recognise rounded hole rims ([#55](https://github.com/stritti/revmesh/issues/55)) ([5590d6a](https://github.com/stritti/revmesh/commit/5590d6a3329a6d55e33a761923046580b7db2f18))
* recognise intersections of outlines extruded along X, Y and Z ([#57](https://github.com/stritti/revmesh/issues/57)) ([8e1185c](https://github.com/stritti/revmesh/commit/8e1185c31825b1e1e5959a84ef1ace69b92079a8))
* recognise outlines stacked along an axis ([#62](https://github.com/stritti/revmesh/issues/62)) ([c0d3ae2](https://github.com/stritti/revmesh/commit/c0d3ae2bdb2c2e15cc4dd84abf1a72c28594ce4d))
* translate the plugin's texts into German (i18n, English default) ([#56](https://github.com/stritti/revmesh/issues/56)) ([15c1653](https://github.com/stritti/revmesh/commit/15c165393c0078f7a0ae9915a5412fc8f99a2571))


### Documentation

* map concepts from the tools and literature to mesh2cad and revmesh ([#53](https://github.com/stritti/revmesh/issues/53)) ([594d055](https://github.com/stritti/revmesh/commit/594d05578b47bb77f8f03b2f1dd5b286e6d521be))

## [0.1.0-beta.4](https://github.com/stritti/revmesh/compare/v0.1.0-beta.3...v0.1.0-beta.4) (2026-11-24)


### Features

* check rebuilds against the source mesh (roadmap 4) ([#45](https://github.com/stritti/revmesh/issues/45)) ([968b92c](https://github.com/stritti/revmesh/commit/968b92ccbdc21d97c58dd838e15b22eacb8d4667))
* **ci:** enforce a &gt;= 80% line-coverage gate on revmesh-core, documented in the agent skills ([#44](https://github.com/stritti/revmesh/issues/44)) ([181e9cd](https://github.com/stritti/revmesh/commit/181e9cd55528d1a036d7620faeaf5ddb88a3f6a0))
* **plate:** recognise rounds at the foot of round bosses ([#51](https://github.com/stritti/revmesh/issues/51)) ([e8c7b36](https://github.com/stritti/revmesh/commit/e8c7b36aebd5a4a9260a5ec1caed56a1949328bf))
* recognise boxes with all twelve edges rounded (edge_fillet_r) ([#47](https://github.com/stritti/revmesh/issues/47)) ([7b678e7](https://github.com/stritti/revmesh/commit/7b678e7ec17e9d9fd2acdce5665743ea55461bf8))
* recognise extrusions along X and Y ([#42](https://github.com/stritti/revmesh/issues/42)) ([a169c0a](https://github.com/stritti/revmesh/commit/a169c0af1c5399f6578b0a8ddc9f1553061fae26))


### Documentation

* add agent skills for SolidCraft WebAssembly plugin development ([#41](https://github.com/stritti/revmesh/issues/41)) ([b2af198](https://github.com/stritti/revmesh/commit/b2af1985328930c3b33c6c60dba5f19ae82e4438))
* list comparable mesh-to-CAD tools with links, licenses and matrix ([#50](https://github.com/stritti/revmesh/issues/50)) ([0c386c4](https://github.com/stritti/revmesh/commit/0c386c4d1a45524b775b50da9712abe811004a29))

## [0.1.0-beta.3](https://github.com/stritti/revmesh/compare/v0.1.0-beta.2...v0.1.0-beta.3) (2026-11-23)


### Bug Fixes

* cut the first beta release under the revmesh name ([#35](https://github.com/stritti/revmesh/issues/35)) ([71412cc](https://github.com/stritti/revmesh/commit/71412cc22c3f84f20ac9a901f4f23f11f79d1e7a))

## [0.1.0-beta.2](https://github.com/stritti/revmesh/compare/v0.1.0-beta.1...v0.1.0-beta.2) (2026-11-22)


### Features

* name parameters as mesh2cad does and keep the part in place ([#9](https://github.com/stritti/revmesh/issues/9)) ([da49758](https://github.com/stritti/revmesh/commit/da49758e90e2e7c9f38cd00923b02fbd959a40d4))
* **plate:** recognise blind holes ([#14](https://github.com/stritti/revmesh/issues/14)) ([a6c067c](https://github.com/stritti/revmesh/commit/a6c067c3e8a3a55f15b9e9062e874b7e43f76ad2))
* **plate:** recognise polygonal cut-outs ([#21](https://github.com/stritti/revmesh/issues/21)) ([ee4bcf6](https://github.com/stritti/revmesh/commit/ee4bcf609203c6dd0fe061069c1ed3f42d930499))
* **plate:** recognise rectangular pads and pockets ([#22](https://github.com/stritti/revmesh/issues/22)) ([ac47e78](https://github.com/stritti/revmesh/commit/ac47e787a20344b6b6c5e27fb326fe2b14f337ad))
* **plate:** recognise round bosses on the top face ([#20](https://github.com/stritti/revmesh/issues/20)) ([f77c985](https://github.com/stritti/revmesh/commit/f77c9850b302991219d06e67ec7eaf5f41a51307))
* **plate:** recognise straight slots ([#15](https://github.com/stritti/revmesh/issues/15)) ([2bc1a2b](https://github.com/stritti/revmesh/commit/2bc1a2bb8e725749963388f62fe86033ecc9939c))
* **recognize:** counterbored and countersunk holes ([#13](https://github.com/stritti/revmesh/issues/13)) ([aa80639](https://github.com/stritti/revmesh/commit/aa80639752a961193fe60db18a53c95b57422be7))
* **recognize:** hole rows, grids and bolt circles as SolidCraft patterns ([#11](https://github.com/stritti/revmesh/issues/11)) ([3c4a64d](https://github.com/stritti/revmesh/commit/3c4a64d0a9fd9a034efe510e0d687d1ca7ee84e6))


### Bug Fixes

* **import:** preserve placement and handle STL input limits ([#32](https://github.com/stritti/revmesh/issues/32)) ([9769821](https://github.com/stritti/revmesh/commit/9769821494416353a5e5596efb91426c6b10ea8d))
* **recognize:** recognise STL files that SolidCraft itself exports ([#12](https://github.com/stritti/revmesh/issues/12)) ([1839760](https://github.com/stritti/revmesh/commit/18397609897781bcdd527dc87731a608b3da27b0))


### Documentation

* explain how to use the plugin in SolidCraft ([#6](https://github.com/stritti/revmesh/issues/6)) ([2147de1](https://github.com/stritti/revmesh/commit/2147de1bc9db6556a5f263d369c628618d7f36ea))
* publish a VitePress site on GitHub Pages ([#10](https://github.com/stritti/revmesh/issues/10)) ([e8a6486](https://github.com/stritti/revmesh/commit/e8a64864f5147c13dc425c249a02d8d6a6f0f780))

## 0.1.0-beta.1 (2026-11-21)


### Features

* project skeleton for the revmesh SolidCraft importer ([66706d6](https://github.com/stritti/revmesh/commit/66706d694a40899bc86997f3fdc193fa72676e23))
* **recognize:** recognise plates with any outline of straight sides ([5e248b4](https://github.com/stritti/revmesh/commit/5e248b45ef2a1034b19e6d5bfae7673d443065d4))
* **recognize:** recognise plates with holes, rounded corners and any straight-sided outline ([9ac04bd](https://github.com/stritti/revmesh/commit/9ac04bd00a83ebc97b8e5c22090fbb06822db883))
* **recognize:** recognise plates with through holes and rounded corners ([24eae8e](https://github.com/stritti/revmesh/commit/24eae8e5f368dd31800bdc567ae169666920635a))


### Bug Fixes

* **recognize:** fit rounded corners robustly in single-precision STL ([533eb14](https://github.com/stritti/revmesh/commit/533eb14d90fa5969e2b54ec2157cc88997e3c737))
* **recognize:** refuse D-shaped cut-outs, fillet before holes, linear outline scan ([f96ce65](https://github.com/stritti/revmesh/commit/f96ce6569224bf04d0119db0afd9ab59dd0e09a2))
* stream large STL files, reject broken ones, own token for release-please ([#5](https://github.com/stritti/revmesh/issues/5)) ([8313750](https://github.com/stritti/revmesh/commit/8313750630eeb66a12b0a54fc4cfbf52b11f2b1c))
