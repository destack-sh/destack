import type { Session, Supergraph } from "@destack/language/core";
import { Region, StructFrozen, StructType } from "@destack/language/core";
import { registerStructClass } from "@destack/language/registry";
import { GalaxyInfoProto, RegionProto } from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashString } from "@destack/utils/hash";

/* ==== DESTACK_GENERATED_START:STRUCT:160101 ==== */
/**
 * GalaxyInfo
 */
export class GalaxyInfo extends StructFrozen {
  static metatype: StructType = StructType.GALAXY_INFO;
  static __isFrozen__: boolean = true;

  /**
   * GalaxyInfo.region
   */
  readonly region: Region;

  /**
   * GalaxyInfo.name
   */
  readonly name: string;

  /**
   * GalaxyInfo.host
   */
  readonly host: string;

  constructor(options: {
    region: Region;
    name: string;
    host: string;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _value?: { [key: string]: any } | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    // properties
    let _region = options.region;
    if (_region === null) {
      throw new Error(`GalaxyInfo.region is required`);
    }
    this.region = _region;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`GalaxyInfo.name is required`);
    }
    this.name = _name;
    let _host = options.host;
    if (_host === null) {
      throw new Error(`GalaxyInfo.host is required`);
    }
    this.host = _host;

    // identity
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._value = options._value ?? null;
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.region === other.region)) {
      return false;
    }
    if (!(this.name === other.name)) {
      return false;
    }
    if (!(this.host === other.host)) {
      return false;
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`region=${Region[this.region]}`);
      propertyReprs.push(`name=${this.name}`);
      propertyReprs.push(`host=${this.host}`);
      // @ts-expect-error(readonly)
      this._repr = `<GalaxyInfo ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash !== null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.region) & 0xffffffff;
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    h = (h * 31 + hashString(this.host)) & 0xffffffff;

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toValue(): { [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = GalaxyInfo.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: GalaxyInfo): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 160101;
    objectValue["50"] = object.region;
    objectValue["51"] = object.name;
    objectValue["52"] = object.host;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): GalaxyInfo {
    return new GalaxyInfo({
      region: Number(objectValue["50"]),
      name: objectValue["51"],
      host: objectValue["52"],
      _value: objectValue,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): GalaxyInfo {
    return GalaxyInfo.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): GalaxyInfoProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = GalaxyInfo.__packProto__(this);
    }
    return this._proto as GalaxyInfoProto;
  }

  static __packProto__(object: GalaxyInfo): GalaxyInfoProto {
    const objectProto: Partial<GalaxyInfoProto> = { metatype: 160101 };
    objectProto.region = Number(object.region) as RegionProto;
    objectProto.name = object.name;
    objectProto.host = object.host;
    return objectProto as GalaxyInfoProto;
  }

  static __unpackProto__(
    objectProto: GalaxyInfoProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): GalaxyInfo {
    return new GalaxyInfo({
      region: Number(objectProto.region) as Region,
      name: objectProto.name,
      host: objectProto.host,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: GalaxyInfoProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): GalaxyInfo {
    return GalaxyInfo.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): GalaxyInfo {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = GalaxyInfoProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.GALAXY_INFO, GalaxyInfo);
/* ==== DESTACK_GENERATED_END:STRUCT:160101 ==== */
