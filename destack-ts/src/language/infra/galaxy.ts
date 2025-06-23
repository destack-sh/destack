import { Region, Session, Struct, StructType, Supergraph } from "@/language";

/* ==== DESTACK_GENERATED_START:STRUCT:7601 ==== */
export class GalaxyInfo extends Struct {
  static metatype: StructType = StructType.GALAXY_INFO;
  static __isFrozen__: boolean = false;

  region: Region;
  name: string;
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
}
/* ==== DESTACK_GENERATED_END:STRUCT:7601 ==== */
