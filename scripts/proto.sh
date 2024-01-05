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
npx protoc --ts_out $TARGET_TS_DIR --ts_opt long_type_string --proto_path . $GENERATED_PROTO_FILE $EXTRA_PROTO_FILES
# prepend every TS file in $TARGET_TS_DIR with /* eslint-disable */
find $TARGET_TS_DIR -type f -name "*.ts" -exec sh -c 'echo "/* eslint-disable */" | cat - "{}" > temp && mv temp "{}"' \;
# re-export everything from the TS files to frontend/wire/index.ts
echo "export * from \"@/proto/wire/bench/proto/bench\";" > $TARGET_TS_DIR/index.ts
