#!/bin/bash

set -e

./scripts/copy_project.sh -s prod -t local -p symbolx/docs
./scripts/copy_project.sh -s prod -t local -p symbolx/examples
