import { CSON_OBJECT_ENCODERS, getObjectKey } from "@destack/encoder/cson/generate";
import { loadEncoders } from "@destack/encoder/cson/generated";
import { packCson, unpackCson } from "@destack/encoder/cson/wiring";
import {
  BuiltinObject,
  Encoder,
  Graph,
  GraphConnection,
  NodeType,
  ObjectKind,
  Session,
  StructType,
  Type,
} from "@destack/language";

/** Encoder for our CSON format. */
export class CsonEncoder implements Encoder<any> {
  constructor() {
    loadEncoders();
  }

  packObject(options: {
    kind: ObjectKind;
    metatype: NodeType | StructType;
    object: BuiltinObject;
  }): any {
    const encoder = CSON_OBJECT_ENCODERS[getObjectKey(options.kind, options.metatype)];
    if (!encoder) {
      throw new Error(
        `no CsonEncoder for ${ObjectKind[options.kind] ?? options.kind}:${NodeType[options.metatype] ?? StructType[options.metatype] ?? options.metatype}`,
      );
    }
    return encoder.packObject(options.object);
  }

  packObjectBytes(options: {
    kind: ObjectKind;
    metatype: NodeType | StructType;
    object: BuiltinObject;
  }): Uint8Array {
    const encoder = CSON_OBJECT_ENCODERS[getObjectKey(options.kind, options.metatype)];
    if (!encoder) {
      throw new Error(
        `no CsonEncoder for ${ObjectKind[options.kind] ?? options.kind}:${NodeType[options.metatype] ?? StructType[options.metatype] ?? options.metatype}`,
      );
    }
    const objectPacked = encoder.packObject(options.object);
    return new TextEncoder().encode(JSON.stringify(objectPacked));
  }

  unpackObject(options: {
    kind: ObjectKind;
    metatype: NodeType | StructType;
    value: any;
    _session: Session | null;
    _graph: Graph | null;
    _connection: GraphConnection | null;
  }): BuiltinObject {
    const encoder = CSON_OBJECT_ENCODERS[getObjectKey(options.kind, options.metatype)];
    if (!encoder) {
      throw new Error(
        `no CsonEncoder for ${ObjectKind[options.kind] ?? options.kind}:${NodeType[options.metatype] ?? StructType[options.metatype] ?? options.metatype}`,
      );
    }
    return encoder.unpackObject(options);
  }

  unpackObjectBytes(options: {
    kind: ObjectKind;
    metatype: NodeType | StructType;
    value: Uint8Array;
    _session: Session | null;
    _graph: Graph | null;
    _connection: GraphConnection | null;
  }): BuiltinObject {
    const encoder = CSON_OBJECT_ENCODERS[getObjectKey(options.kind, options.metatype)];
    if (!encoder) {
      throw new Error(
        `no CsonEncoder for ${ObjectKind[options.kind] ?? options.kind}:${NodeType[options.metatype] ?? StructType[options.metatype] ?? options.metatype}`,
      );
    }
    const objectPacked = JSON.parse(new TextDecoder().decode(options.value));
    return encoder.unpackObject({
      ...options,
      value: objectPacked,
    });
  }

  packValue(options: { value: any; type: Type }): any {
    const cson = packCson(options.value, options.type);
    return cson;
  }

  packValueBytes(options: { value: any; type: Type }): Uint8Array {
    const cson = packCson(options.value, options.type);
    return new TextEncoder().encode(JSON.stringify(cson));
  }

  unpackValue(options: { type: Type; value: any }): any {
    const cson = unpackCson(options.value, options.type);
    return cson;
  }

  unpackValueBytes(options: { type: Type; value: Uint8Array }): any {
    const cson = unpackCson(options.value, options.type);
    return cson;
  }
}
