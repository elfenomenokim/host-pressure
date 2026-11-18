# README diagrams

Mermaid sources are rendered to transparent SVG in English and Korean.
`light.json` and `dark.json` set the colors and text style. Render from the
repository root with mmdc and a dedicated headless shell:

```sh
HEADLESS_SHELL=/path/to/headless_shell bash docs/diagrams/render.sh
```

The renderer passes an explicit Puppeteer executable path; it must not launch
a personal browser. Inspect the SVGs at the README's displayed width. The
Korean README embeds the `.ko.svg` copies. The public README uses absolute
raw URLs so the rendered diagrams can appear on PyPI as images.

`benchmark_chart.py` regenerates the chart's Mermaid sources directly from
`docs/bench/m5-no-sample.json`, `a1-no-sample.json`, and
`windows-no-sample.json`. Values are means over 30 calls after 3 warmups;
the x-axis preserves the Python/Rust order for each host. Different hosts use
the launch methods documented beside those raw files.
