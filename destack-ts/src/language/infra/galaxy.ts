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

    this.region = options.region;
    this.name = options.name;
    this.host = options.host;
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
} /* ==== DESTACK_GENERATED_END:STRUCT:7601 ==== */
