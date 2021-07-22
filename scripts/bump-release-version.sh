#!/bin/sh

# One-liner to update between ink! release versions.
#
# Usage: ./bump-release-version.sh 3.0.0-rc3 3.0.0-rc4

set -xeu

OLD_VERSION=$1
NEW_VERSION=$2

find . -type f -name 'Cargo.toml' -exec sed -i -e "s/$OLD_VERSION/$NEW_VERSION/g" {} \;
