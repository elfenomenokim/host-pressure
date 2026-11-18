# Demo recording

Run from the repository root with Python 3, ffmpeg, and `vhs-safe` on PATH.
`vhs-safe` is the maintainer's patched VHS v0.12.1 launcher: it uses a dedicated
headless shell through `VHS_BROWSER`. Stock VHS v0.12.1 ignored `rod=bin=...`
and launched Google Chrome, so it is not an interchangeable recorder.

```sh
HEADLESS_SHELL=/path/to/dedicated/headless_shell bash docs/demo/record.sh
```

The tapes use Catppuccin Latte/Mocha, 1200×620, 18-point text, and 45 ms typing.
The script runs the two recordings sequentially and extracts each final frame.
Inspect both PNGs before publishing. If `pgrep -f 'vhs-'` reports anything,
identify it and clean up only processes this recording started.

`fixture.py` supplies synthetic kernel readings to the actual Python CLI.
Its values are demonstration inputs, not measured performance. The installed
command and library are unchanged.

`frame.png` is the inspected final light frame. `social.html` uses it in the
shared 1280×640 preview layout. To render that page, run a dedicated headless
shell directly with `--window-size=1280,640 --force-device-scale-factor=1`,
`--screenshot=docs/social.png`, and the absolute file URL of `social.html`.
No signed-in browser is needed. The README's remote image URLs become live
only after the maintainer exports and publishes these files.
