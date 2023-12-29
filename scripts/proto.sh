set -e

TARGET_PY_PATH="bench/proto/wire"
TARGET_TS_DIR="frontend/src/proto/wire"
GENERATED_PROTO_FILE="bench/proto/bench.proto"
EXTRA_PROTO_FILES="bench/proto/services.proto"

# generate proto file
python manage.py proto --file $GENERATED_PROTO_FILE

# python
rm -rf $TARGET_PY_PATH
mkdir -p $TARGET_PY_PATH
echo "generate $TARGET_PY_PATH"
protoc -I . --python_betterproto_out=$TARGET_PY_PATH $GENERATED_PROTO_FILE $EXTRA_PROTO_FILES
mv $TARGET_PY_PATH/__init__.py $TARGET_PY_PATH.py
rm -r $TARGET_PY_PATH

# TS
rm -rf $TARGET_TS_DIR
echo "generate $TARGET_TS_DIR"
mkdir -p $TARGET_TS_DIR
protoc --plugin=./node_modules/.bin/protoc-gen-ts_proto \
 --ts_proto_opt=oneof=unions \
 --ts_proto_opt=removeEnumPrefix=true \
 --ts_proto_opt=unrecognizedEnum=false \
 --ts_proto_out=$TARGET_TS_DIR $GENERATED_PROTO_FILE
# re-export everything from the TS files to frontend/wire/index.ts
echo "export * from \"@/proto/wire/bench/proto/bench\";" > $TARGET_TS_DIR/index.ts
