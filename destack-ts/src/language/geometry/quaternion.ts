import type { Graph, GraphConnection, Session, Supergraph } from "@destack/language/core";
import { StructType } from "@destack/language/core";
import { Vector4 } from "@destack/language/geometry/vector";
import { registerStructClass } from "@destack/language/registry";
import { QuaternionProto } from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashFloat } from "@destack/utils/hash";

/* ==== DESTACK_GENERATED_START:STRUCT:2400010 ==== */
/**
 * A quaternion.
 */
export class Quaternion extends Vector4 {
  static metatype: StructType = StructType.QUATERNION;
  static __isFrozen__: boolean = true;

  constructor(options: {
    x: number;
    y: number;
    z: number;
    w: number;
    _session?: Session | null;
    _graph?: Supergraph | null;
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _cson?: any | null;
  }) {
    super(options);

    // properties

    // identity
    // ... (already set in parent)
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.x === other.x || Math.abs(this.x - other.x) < 1e-10)) {
      return false;
    }
    if (!(this.y === other.y || Math.abs(this.y - other.y) < 1e-10)) {
      return false;
    }
    if (!(this.z === other.z || Math.abs(this.z - other.z) < 1e-10)) {
      return false;
    }
    if (!(this.w === other.w || Math.abs(this.w - other.w) < 1e-10)) {
      return false;
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`x=${this.x}`);
      propertyReprs.push(`y=${this.y}`);
      propertyReprs.push(`z=${this.z}`);
      propertyReprs.push(`w=${this.w}`);
      // @ts-expect-error(readonly)
      this._repr = `<Quaternion ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash != null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashFloat(this.x)) & 0xffffffff;
    h = (h * 31 + hashFloat(this.y)) & 0xffffffff;
    h = (h * 31 + hashFloat(this.z)) & 0xffffffff;
    h = (h * 31 + hashFloat(this.w)) & 0xffffffff;

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toCson(): { [key: string]: any } {
    if (this._cson === null) {
      // @ts-expect-error(readonly)
      this._cson = Quaternion.__packCson__(this);
    }
    return this._cson;
  }

  static __packCson__(object: Quaternion): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 2400010;
    objectCson["101"] = object.x;
    objectCson["102"] = object.y;
    objectCson["103"] = object.z;
    objectCson["104"] = object.w;
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
    _session?: Session | null,
    _graph?: Graph | null,
    _connection?: GraphConnection | null,
  ): Quaternion {
    return new Quaternion({
      x: objectCson["101"],
      y: objectCson["102"],
      z: objectCson["103"],
      w: objectCson["104"],
      _cson: objectCson,
      _graph,
    });
  }

  static fromCson(
    objectCson: { readonly [key: string]: any },
    _session?: Session | null,
    _graph?: Graph | null,
    _connection?: GraphConnection | null,
  ): Quaternion {
    return Quaternion.__unpackCson__(objectCson, _session, _graph, _connection);
  }

  toProto(): QuaternionProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Quaternion.__packProto__(this);
    }
    return this._proto as QuaternionProto;
  }

  static __packProto__(object: Quaternion): QuaternionProto {
    const objectProto: Partial<QuaternionProto> = { metatype: 2400010 };
    objectProto.x = object.x;
    objectProto.y = object.y;
    objectProto.z = object.z;
    objectProto.w = object.w;
    return objectProto as QuaternionProto;
  }

  static __unpackProto__(
    objectProto: QuaternionProto,
    _session?: Session | null,
    _graph?: Supergraph | null,
    _connection?: GraphConnection | null,
  ): Quaternion {
    return new Quaternion({
      x: objectProto.x,
      y: objectProto.y,
      z: objectProto.z,
      w: objectProto.w,
      _proto: objectProto,
      _graph,
    });
  }

  static fromProto(
    objectProto: QuaternionProto,
    _session?: Session | null,
    _graph?: Supergraph | null,
    _connection?: GraphConnection | null,
  ): Quaternion {
    return Quaternion.__unpackProto__(objectProto, _session, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Quaternion {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = QuaternionProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.QUATERNION, Quaternion);
/* ==== DESTACK_GENERATED_END:STRUCT:2400010 ==== */
