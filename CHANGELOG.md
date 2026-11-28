# Changelog

## [0.3.0](https://github.com/stritti/revmesh/compare/v0.2.0...v0.3.0) (2026-11-28)


### Features

* **ci:** headless CLI example — install dist zip, file.import an example STL, verify rebuilt volume ([#104](https://github.com/stritti/revmesh/issues/104)) ([efb9f6b](https://github.com/stritti/revmesh/commit/efb9f6be343a3e627b05b9bfefed2d91d5847f65))
* **design:** mesh coordinates beyond f32 range keep double precision (roadmap 7.3) ([#120](https://github.com/stritti/revmesh/issues/120)) ([052296a](https://github.com/stritti/revmesh/commit/052296aa3c847b633654aed6c2a28d28df5de04b))
* drop the auto/mesh import choice; the plugin picks the result itself ([#114](https://github.com/stritti/revmesh/issues/114)) ([8da8de1](https://github.com/stritti/revmesh/commit/8da8de1fa84a9bfff372b142b69c0d27260687d3))
* **import:** raise the auto ceiling to 16 MiB, localise STL syntax errors, add a units option ([#97](https://github.com/stritti/revmesh/issues/97)) ([f1f72a7](https://github.com/stritti/revmesh/commit/f1f72a77a618c7123982191407d77b72be2fdd40))
* OBJ and PLY input next to STL (roadmap 7.2) ([#105](https://github.com/stritti/revmesh/issues/105)) ([c54cbcb](https://github.com/stritti/revmesh/commit/c54cbcbb967241997fb0c0a98fdfd73577aad602))
* **patterns:** hole rows at any angle are written as single holes (roadmap 6.5) ([#119](https://github.com/stritti/revmesh/issues/119)) ([e56a99a](https://github.com/stritti/revmesh/commit/e56a99a28336a3f36403921153b89c196d1727d4))
* per-corner radii in outlines (roadmap 6.4) ([#110](https://github.com/stritti/revmesh/issues/110)) ([3235183](https://github.com/stritti/revmesh/commit/3235183eed2da999884c4b21177e26d4b13917de))
* **plugin:** also place the stl importer in the Insert ribbon tab ([#95](https://github.com/stritti/revmesh/issues/95)) ([2b1383f](https://github.com/stritti/revmesh/commit/2b1383f584cdcc488444ae0aaa4ed96402c3335b))
* **plugin:** show the project link in the plugin card description ([#98](https://github.com/stritti/revmesh/issues/98)) ([cb9f702](https://github.com/stritti/revmesh/commit/cb9f7025a9edd778c94837cbd0040b658ca2eb43))
* recognise prisms along an arbitrary axis by RANSAC (roadmap 6.1) ([#117](https://github.com/stritti/revmesh/issues/117)) ([99d06dd](https://github.com/stritti/revmesh/commit/99d06ddbebd94cde5634fecda7012f7ddb21c2e4))
* **recognize:** ComplexGen-style topology gates — closed edge loops, walls at 90° ([#101](https://github.com/stritti/revmesh/issues/101)) ([53a0cc4](https://github.com/stritti/revmesh/commit/53a0cc472a4949ccdc7c5981c29c4ad1d491d3c2))
* **recognize:** crossing holes in combined parts, roofs and bevels on stacks (roadmap 3.12) ([#118](https://github.com/stritti/revmesh/issues/118)) ([5d3857d](https://github.com/stritti/revmesh/commit/5d3857d837e3ed74ebe88d4dfc4d150375b4232e))
* round bosses on side walls (roadmap 3.10) ([#112](https://github.com/stritti/revmesh/issues/112)) ([dbcf20b](https://github.com/stritti/revmesh/commit/dbcf20b139ff50af7e829b0307c8bf6742fea6eb))
* **scripts:** add scripts/measure-stl.py and document script-based install in README/docs ([#108](https://github.com/stritti/revmesh/issues/108)) ([93ea701](https://github.com/stritti/revmesh/commit/93ea701c7eb2a1a60e08f5c86eb6c9380fd098b2))


### Bug Fixes

* **import:** preserve concave faces and wide mesh coordinates ([#121](https://github.com/stritti/revmesh/issues/121)) ([b254c9c](https://github.com/stritti/revmesh/commit/b254c9c28d91537c871fea443bf6b36fd05fb20b))
* **plate:** recognize subdivided through-hole walls ([#123](https://github.com/stritti/revmesh/issues/123)) ([3510e00](https://github.com/stritti/revmesh/commit/3510e009969469b17be9d183e94f1902cbcfa38c))
* **recognize:** one two-pointer sweep over a line's events in closed_loops ([#107](https://github.com/stritti/revmesh/issues/107)) ([0830c59](https://github.com/stritti/revmesh/commit/0830c59955e172d74b3761febdbc58b69756d64f))


### Documentation

* add DE/EN locales to the docs site and enlarge the home hero image ([#96](https://github.com/stritti/revmesh/issues/96)) ([f502bf8](https://github.com/stritti/revmesh/commit/f502bf867ed943d4c3ede481acb1e3961e977ef9))
* add hero image so the plugin-list card has a picture ([#115](https://github.com/stritti/revmesh/issues/115)) ([b5ee0f1](https://github.com/stritti/revmesh/commit/b5ee0f1def36014135264b3f37035037cce98e4e))
* add research/mesh2cad-parity.md and fix stale 8 MiB ceiling in DE docs ([#103](https://github.com/stritti/revmesh/issues/103)) ([7335b9e](https://github.com/stritti/revmesh/commit/7335b9ee94cd2f1b5aab666a39535e18845daacd))
* document SolidCraft compatibility and runtime limits in README and docs site ([#100](https://github.com/stritti/revmesh/issues/100)) ([d93c5df](https://github.com/stritti/revmesh/commit/d93c5df8964fcf8b014a0b9871ba2fe8bc9bfbe5))
* move the plugin-list mention from the hero button to the footer ([#116](https://github.com/stritti/revmesh/issues/116)) ([8d37b94](https://github.com/stritti/revmesh/commit/8d37b94e0c12764dc18ef10cb4270832aee8f87a))
* spike for issue [#70](https://github.com/stritti/revmesh/issues/70) — convert a mesh body in the open design ([#109](https://github.com/stritti/revmesh/issues/109)) ([d41c52c](https://github.com/stritti/revmesh/commit/d41c52c2d0c97a2f084d38e29892daeaa3f18458))

## [0.2.0](https://github.com/stritti/revmesh/compare/v0.1.0...v0.2.0) (2026-11-26)


### Features

* **design:** dimension the stack's and combined's sketches ([#84](https://github.com/stritti/revmesh/issues/84)) ([dd18ad9](https://github.com/stritti/revmesh/commit/dd18ad940f437ad87a59cab412b23ac64765d949))
* **plugin:** check the recognised design in the app's sandbox before writing it ([#90](https://github.com/stritti/revmesh/issues/90)) ([322e7d7](https://github.com/stritti/revmesh/commit/322e7d7d8a92f553e14465eb87416d218cfa43d4))
* **plugin:** name the GitHub repository in plugin.json for SolidCraft's plugin card ([#87](https://github.com/stritti/revmesh/issues/87)) ([7d87983](https://github.com/stritti/revmesh/commit/7d87983b328df6c9bd563c86e29725a45912f28c))
* **plugin:** spell out the auto/mesh differences in the import dialog ([#91](https://github.com/stritti/revmesh/issues/91)) ([a8ad87e](https://github.com/stritti/revmesh/commit/a8ad87ebd99fcf81dc1db00b487fd4dbfb58ccf7))
* **recognize:** arc sides in cut-outs ([#82](https://github.com/stritti/revmesh/issues/82)) ([065147c](https://github.com/stritti/revmesh/commit/065147c8dba08fb9a895ab64e3e82c34064de80f))
* **recognize:** fit rim and foot rounds as tori in the profile plane ([#89](https://github.com/stritti/revmesh/issues/89)) ([71a13fe](https://github.com/stritti/revmesh/commit/71a13fe3a91b4d343585139b4539d2836c502a9d))
* **recognize:** headed holes opening on a step's level ([#85](https://github.com/stritti/revmesh/issues/85)) ([246db61](https://github.com/stritti/revmesh/commit/246db61a19d1afe285c80d643168c5947c0de281))
* **recognize:** robust circle fits for rim loops via fixed-seed RANSAC ([#88](https://github.com/stritti/revmesh/issues/88)) ([a2b4205](https://github.com/stritti/revmesh/commit/a2b42053d43b8aae8fc00ecdb96a1fd6b3f2b689))
* **report:** deviation report for combined, stacked and all-over-rounded parts ([#86](https://github.com/stritti/revmesh/issues/86)) ([8565616](https://github.com/stritti/revmesh/commit/8565616f9d4a976f1496964cc4846f699fac7713))


### Bug Fixes

* [ImgBot] Optimize images ([#93](https://github.com/stritti/revmesh/issues/93)) ([dcac8cd](https://github.com/stritti/revmesh/commit/dcac8cdbb5f6047f0e4e9e92acc3f61599b27c06))


### Documentation

* animated STL-to-design SVG as the home page hero ([#92](https://github.com/stritti/revmesh/issues/92)) ([27a0157](https://github.com/stritti/revmesh/commit/27a0157d6703fe7945c70d41e1539b1a45858b2d))
* make the home page benefit-first, move technical detail behind links ([#94](https://github.com/stritti/revmesh/issues/94)) ([81fab07](https://github.com/stritti/revmesh/commit/81fab072b52e8eb3cc1f0ebcf5e8a0d4ce03616e))
* **skills:** always check SolidCraft plugin-interface compatibility and full utilization ([#81](https://github.com/stritti/revmesh/issues/81)) ([6b0c962](https://github.com/stritti/revmesh/commit/6b0c9628444977180ee5a32c9389bbefd50e418e))

## [0.1.0](https://github.com/stritti/revmesh/compare/v0.1.0-beta.6...v0.1.0) (2026-11-25)


### Documentation

* **release:** note that a release PR needs a user-facing commit since the last release ([#78](https://github.com/stritti/revmesh/issues/78)) ([08a7d9d](https://github.com/stritti/revmesh/commit/08a7d9d519dced140aad23091bd73b0bcab0c3e1))

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
