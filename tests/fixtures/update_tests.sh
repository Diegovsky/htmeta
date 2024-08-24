#!/usr/bin/env sh
cd $(realpath $(dirname $0))
set -euo pipefail

if test $# -eq 0; then
    files=$(ls *.kdl)
else
    shift 1
    files=$@
fi

for x in $files; do
    flags=""
    case $x in
        minified_*)
            flags="$flags -m"
            ;;
    esac
    cargo run -p htmeta-cli -- $flags $x
done

wait
