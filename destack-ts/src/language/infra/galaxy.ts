import { Region, Session, Struct, StructType, Supergraph } from "@destack/language/core";

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
    throw new Error("not implemented");
  }

  hash(): number {
    throw new Error("not implemented");
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
}
/* ==== DESTACK_GENERATED_END:STRUCT:7601 ==== */
