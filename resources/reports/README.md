# Offline report resources

The RxMER capture is a byte-for-byte copy of the sanitized PyPNM-derived
`tests/fixtures/pypnm/tests/files/rxmer.bin`. Preserve its existing fixture
license/provenance in `tests/fixtures/pypnm/`. No customer MAC data is added.

SHA-256: `864ba70b0361bd8b4912b55f55e8db72dd7b319b478ce1988103979e0b19c98e`.

JSON reports are project-owned GPL-3.0-only demonstrations. Profile tables are
explicitly synthetic; the chart and sample-count evidence use parsed PNM values.
Image IDs resolve existing bundled branding assets. None requires network access.

`example.jpg` and `example.webp` are deterministic format variants of existing
`docs/images/raven-icon-dark.png`, made with Pillow 90-quality RGB JPEG
(subsampling 0) and lossless WebP (method 6). They introduce no generated or
third-party artwork. Qt runtime plugins own decoding; unsupported formats have
an explicit unavailable state. Neither resource is loaded from user/model paths.
