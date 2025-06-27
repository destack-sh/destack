import { Session, Supergraph } from "@destack/language/core";
import { Region, Struct, StructType } from "@destack/language/core/builtin";
import { registerStructClass } from "@destack/language/registry";
import { GalaxyInfoProto, RegionProto } from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashString } from "@destack/utils/hash";

/* ==== DESTACK_GENERATED_START:STRUCT:7601 ==== */
/**
 * GalaxyInfo
 */
export class GalaxyInfo extends Struct {
  static metatype: StructType = StructType.GALAXY_INFO;
  static __isFrozen__: boolean = false;

  /**
   * GalaxyBase.region
   */
  region: Region;

  /**
   * GalaxyBase.name
   */
  name: string;

  /**
   * GalaxyBase.host
   */
  host: string;

  constructor(options: {
    region: Region;
    name: string;
    host: string;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
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
    // ...
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
    const propertyReprs: string[] = [];
    propertyReprs.push(`region=${Region[this.region]}`);
    propertyReprs.push(`name=${this.name}`);
    propertyReprs.push(`host=${this.host}`);
    return `<GalaxyInfo ${propertyReprs.join(" ")}>`;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.region) & 0xffffffff;
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    h = (h * 31 + hashString(this.host)) & 0xffffffff;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toValue(): { [key: string]: any } {
    return GalaxyInfo.__packValue__(this);
  }

  static __packValue__(object: GalaxyInfo): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 7601;
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
    return GalaxyInfo.__packProto__(this);
  }

  static __packProto__(object: GalaxyInfo): GalaxyInfoProto {
    const objectProto: Partial<GalaxyInfoProto> = { metatype: 7601 };
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
/* ==== DESTACK_GENERATED_END:STRUCT:7601 ==== */
