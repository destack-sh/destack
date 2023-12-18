TARGET_PY_DIR="bench/language/wire"
TARGET_TS_DIR="frontend/src/wire"
PROTO_FILE="bench/language/bench.proto"

# python
rm -rf $TARGET_PY_DIR
mkdir -p $TARGET_PY_DIR
echo "generate $TARGET_PY_DIR"
protoc -I . --python_betterproto_out=$TARGET_PY_DIR bench/language/bench.proto
mv $TARGET_PY_DIR/__init__.py $TARGET_PY_DIR.py

# TS
rm -rf $TARGET_TS_DIR
echo "generate $TARGET_TS_DIR"
mkdir -p $TARGET_TS_DIR
protoc --plugin=./node_modules/.bin/protoc-gen-ts_proto --ts_proto_out=$TARGET_TS_DIR $PROTO_FILE
# re-export everything from the TS files to frontend/wire/index.ts
echo "export * from \"./bench/language/bench\";" >$TARGET_TS_DIR/index.ts
