"""Allow `python -m host_pressure`."""

from .cli import main

if __name__ == "__main__":
    raise SystemExit(main())
