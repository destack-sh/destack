#!/bin/bash
tokei \
	-e '*.yaml' \
	-e '*.json' \
	-e 'proto/language.proto' \
	-e '**/proto/wire/**/*.ts' \
	-e '*.css' \
	-e '*.txt' \
	-e 'bench/migrations' \
	-e 'bench/pb2/*.py'
