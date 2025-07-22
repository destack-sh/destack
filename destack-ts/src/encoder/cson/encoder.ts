import { CSON_OBJECT_ENCODERS, getObjectKey } from "@destack/encoder/cson/generate";
import type {
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
  packObject(options: {
    kind: ObjectKind;
    metatype: NodeType | StructType;
    object: BuiltinObject;
  }): any {
    const encoder = CSON_OBJECT_ENCODERS[getObjectKey(options.kind, options.metatype)];
    return encoder.packObject(options.object);
  }

  packObjectBytes(options: {
    kind: ObjectKind;
    metatype: NodeType | StructType;
    object: BuiltinObject;
  }): Uint8Array {
    const encoder = CSON_OBJECT_ENCODERS[getObjectKey(options.kind, options.metatype)];
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
    const objectPacked = JSON.parse(new TextDecoder().decode(options.value));
    return encoder.unpackObject({
      ...options,
      value: objectPacked,
    });
  }

  packValue(options: { value: any; type: Type }): any {
    throw new Error("not implemented");
  }

  packValueBytes(options: { value: any; type: Type }): Uint8Array {
    throw new Error("not implemented");
  }

  unpackValue(options: { type: Type; value: any }): any {
    throw new Error("not implemented");
  }

  unpackValueBytes(options: { type: Type; value: Uint8Array }): any {
    throw new Error("not implemented");
  }
}
