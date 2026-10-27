# Files of the demo

`just demo` puts these files into the volume `unity.default.images`, for the file viewer of the web
UI (`examples/seed_demo.py`).

- `uc-rs-logo.png`: `docs/assets/logo-light.svg` as a PNG, 600 × 400:
  `rsvg-convert -w 600 docs/assets/logo-light.svg -o examples/demo-files/uc-rs-logo.png`. The
  logo is upstream's Unity Catalog logo (Apache-2.0) with Ferris (CC0).
