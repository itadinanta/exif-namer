# Test fixture attribution and licensing

Files in this directory are third-party test data unless stated otherwise.
Provenance was verified on 2026-11-17 by matching SHA-256 digests against both
the published crate and the upstream repository.

## exif-rs fixtures: `exif.heic`, `exif.jpg`, `exif.png`, `exif.tif`, `exif.webp`, `yaminabe.tif`, `yaminale.tif`

Source crate: [`kamadak-exif` 0.6.2](https://crates.io/crates/kamadak-exif/0.6.2)
(tarball SHA-256
`8772c92c0f41c4a672cbdee6728253e5ae8fcc1c5eb5c6c676579565684bbf68`),
files under `tests/`.

Source repository: <https://github.com/kamadak/exif-rs>,
`master` at
[`e81442a9673e2fba51b2b1d68c63ad779bae8cd2`](https://github.com/kamadak/exif-rs/tree/e81442a9673e2fba51b2b1d68c63ad779bae8cd2).
The files in this directory are byte-identical to the blobs in that tree:

| File | Upstream blob (SHA-1) | Added upstream |
|---|---|---|
| [`exif.heic`](https://github.com/kamadak/exif-rs/blob/e81442a9673e2fba51b2b1d68c63ad779bae8cd2/tests/exif.heic) | `63a680a60f3c5b3c308108d85cb17fcdfee69044` | [`0e398cd`](https://github.com/kamadak/exif-rs/commit/0e398cd) (2020-01-19) |
| [`exif.jpg`](https://github.com/kamadak/exif-rs/blob/e81442a9673e2fba51b2b1d68c63ad779bae8cd2/tests/exif.jpg) | `aa91bb21c2668d6059cc538782ed0cfb316f0095` | [`92a3c49`](https://github.com/kamadak/exif-rs/commit/92a3c49) (2016-12-18) |
| [`exif.png`](https://github.com/kamadak/exif-rs/blob/e81442a9673e2fba51b2b1d68c63ad779bae8cd2/tests/exif.png) | `fea60b1e6fd4e79c3706c1a156ac995b7e9f98db` | [`6e3c37d`](https://github.com/kamadak/exif-rs/commit/6e3c37d) (2020-08-03) |
| [`exif.tif`](https://github.com/kamadak/exif-rs/blob/e81442a9673e2fba51b2b1d68c63ad779bae8cd2/tests/exif.tif) | `9e23505d5970c3edacc165fd8de8b8774083adce` | [`8d4d3d4`](https://github.com/kamadak/exif-rs/commit/8d4d3d4) (2017-03-05) |
| [`exif.webp`](https://github.com/kamadak/exif-rs/blob/e81442a9673e2fba51b2b1d68c63ad779bae8cd2/tests/exif.webp) | `6481e7250dcff68c158512549e316a86cba698c6` | [`f57a972`](https://github.com/kamadak/exif-rs/commit/f57a972) (2020-12-14) |
| [`yaminabe.tif`](https://github.com/kamadak/exif-rs/blob/e81442a9673e2fba51b2b1d68c63ad779bae8cd2/tests/yaminabe.tif) | `2853d2700b6ed1479364d414f7d0d6a1ecd511c7` | [`8e9ff50`](https://github.com/kamadak/exif-rs/commit/8e9ff50) (2019-05-08) |
| [`yaminale.tif`](https://github.com/kamadak/exif-rs/blob/e81442a9673e2fba51b2b1d68c63ad779bae8cd2/tests/yaminale.tif) | `222e60c46c80a61d47e348f35714838f15adb254` | [`17344ad`](https://github.com/kamadak/exif-rs/commit/17344ad) (2021-05-04) |

All were committed to upstream by KAMADA Ken'ichi, the exif-rs author. They
are used here under the exif-rs license, BSD-2-Clause:

> Copyright (c) 2016-2023 KAMADA Ken'ichi.
> All rights reserved.
>
> Redistribution and use in source and binary forms, with or without
> modification, are permitted provided that the following conditions are
> met:
>
> 1. Redistributions of source code must retain the above copyright notice,
>    this list of conditions and the following disclaimer.
> 2. Redistributions in binary form must reproduce the above copyright
>    notice, this list of conditions and the following disclaimer in the
>    documentation and/or other materials provided with the distribution.
>
> THIS SOFTWARE IS PROVIDED BY THE AUTHOR AND CONTRIBUTORS ``AS IS'' AND
> ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
> IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
> ARE DISCLAIMED. IN NO EVENT SHALL THE AUTHOR OR CONTRIBUTORS BE LIABLE FOR
> ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
> DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
> SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
> CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
> LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
> OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH
> DAMAGE.

Verification hashes (SHA-256) of the copies in this directory:

```
6feefa4f4631258be2daad9f92381615417c1acb387a8ca0075968fa2f21a0e5  exif.heic
cd243e5d55636e7c5b73325d28105acbb044b6c49087bd218a114c588bf814e4  exif.jpg
aa0acbed88fd5162587945726832e8967532f4fc860f0e76d73df557ab768ddb  exif.png
0fe205445c995ed0291ae80f0748dda24b02931190ff734139e61dd8660e20de  exif.tif
17dc81ddbc991f1f5e98cb74b61e9661036823ead13fc141e50a6daf993b0177  exif.webp
550a359305063479407a2e617ec2cb7c18bc28548bd7b755e4eae0f06e532267  yaminabe.tif
214a8055635d0da2346d8054449b6baaaa63866937efc905b2c8a85b0924acdd  yaminale.tif
```

## Canon RAW fixtures: `exif.cr2`, `exif.cr3`

These are photographs taken by Nicola Orrù, maintainer of exif-namer. They
are dual-licensed, at the recipient's option, under the **Apache License 2.0**
(the license of this repository) or the **Creative Commons
Attribution-NonCommercial 4.0 International (CC BY-NC 4.0)**:

> exif-namer test fixtures (`exif.cr2`, `exif.cr3`) by Nicola Orrù —
> <https://github.com/itadinanta/exif-namer> — licensed under
> [Apache-2.0](https://www.apache.org/licenses/LICENSE-2.0) or
> [CC BY-NC 4.0](https://creativecommons.org/licenses/by-nc/4.0/), at your
> option.

- Apache-2.0: <https://www.apache.org/licenses/LICENSE-2.0>
- CC BY-NC deed: <https://creativecommons.org/licenses/by-nc/4.0/>
- CC BY-NC legal code: <https://creativecommons.org/licenses/by-nc/4.0/legalcode>

Under the CC BY-NC option, copies and redistributions require attribution to
Nicola Orrù and are limited to non-commercial purposes. Under the Apache-2.0
option, the terms of that license apply, including its attribution and
NOTICE requirements.

They were first published as `tests/exif.cr2` and `tests/exif.cr3` in
[`itadinanta/exif-rs` commit `ffd1141`](https://github.com/itadinanta/exif-rs/commit/ffd1141).
