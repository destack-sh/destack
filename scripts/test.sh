#!/bin/bash
set -e

ENVIRONMENT=test

cargo test

bun run test