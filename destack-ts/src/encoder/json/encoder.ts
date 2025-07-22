import { JSON_OBJECT_ENCODERS, getObjectKey } from "@destack/encoder/json/generate";
import { loadEncoders } from "@destack/encoder/json/generated";
import { packJson, unpackJson } from "@destack/encoder/json/wiring";
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

/** Encoder for our JSON format. */
export class JsonEncoder implements Encoder<any> {
  constructor() {
    loadEncoders();
  }

  packObject(options: {
    kind: ObjectKind;
    metatype: NodeType | StructType;
    object: BuiltinObject;
  }): any {
    const encoder = JSON_OBJECT_ENCODERS[getObjectKey(options.kind, options.metatype)];
    if (!encoder) {
      throw new Error(
        `no JsonEncoder for ${ObjectKind[options.kind] ?? options.kind}:${NodeType[options.metatype] ?? StructType[options.metatype] ?? options.metatype}`,
      );
    }
    return encoder.packObject(options.object);
  }

  packObjectBytes(options: {
    kind: ObjectKind;
    metatype: NodeType | StructType;
    object: BuiltinObject;
  }): Uint8Array {
    const encoder = JSON_OBJECT_ENCODERS[getObjectKey(options.kind, options.metatype)];
    if (!encoder) {
      throw new Error(
        `no JsonEncoder for ${ObjectKind[options.kind] ?? options.kind}:${NodeType[options.metatype] ?? StructType[options.metatype] ?? options.metatype}`,
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
    const encoder = JSON_OBJECT_ENCODERS[getObjectKey(options.kind, options.metatype)];
    if (!encoder) {
      throw new Error(
        `no JsonEncoder for ${ObjectKind[options.kind] ?? options.kind}:${NodeType[options.metatype] ?? StructType[options.metatype] ?? options.metatype}`,
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
    const encoder = JSON_OBJECT_ENCODERS[getObjectKey(options.kind, options.metatype)];
    if (!encoder) {
      throw new Error(
        `no JsonEncoder for ${ObjectKind[options.kind] ?? options.kind}:${NodeType[options.metatype] ?? StructType[options.metatype] ?? options.metatype}`,
      );
    }
    const objectPacked = JSON.parse(new TextDecoder().decode(options.value));
    return encoder.unpackObject({
      ...options,
      value: objectPacked,
    });
  }

  packValue(options: { value: any; type: Type }): any {
    const json = packJson(options.value, options.type);
    return json;
  }

  packValueBytes(options: { value: any; type: Type }): Uint8Array {
    const json = packJson(options.value, options.type);
    return new TextEncoder().encode(JSON.stringify(json));
  }

  unpackValue(options: { type: Type; value: any }): any {
    const json = unpackJson(options.value, options.type);
    return json;
  }

  unpackValueBytes(options: { type: Type; value: Uint8Array }): any {
    const jsonValue = JSON.parse(new TextDecoder().decode(options.value));
    const json = unpackJson(jsonValue, options.type);
    return json;
  }
} 