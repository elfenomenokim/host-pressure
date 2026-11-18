# Sourced by VHS while hidden; this function selects the documented demo fixture.
export PS1='$ '
export XDG_CONFIG_HOME="$(mktemp -d /tmp/host-pressure-demo.XXXXXX)"
host-pressure() { python3 docs/demo/fixture.py "$@"; }
clear
