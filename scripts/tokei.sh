#!/bin/bash
tokei \
	-e '*.yaml' \
	-e '*.json' \
	-e '*.css' \
	-e '*.txt' \
	-e 'bench-proto/language.proto' \
	-e '**/bench-proto/wire/**/*.ts' \
	-e 'bench-py/bench/migrations' \
	-e 'bench-py/bench/pb2/*.py'
