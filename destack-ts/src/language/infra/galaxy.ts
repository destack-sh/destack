import { EnumType, Node, Supergraph, activeSession, Session, Region, NodeType, Struct, QueryConnection, StructFrozen, ACTIVE_SESSION, StructType, NodeReference, BuiltinObject, Graph } from '@/language';

/* ==== DESTACK_GENERATED_START:STRUCT:7601 ==== */
export class GalaxyInfo extends Struct {
  region: Region;
  name: string;
  host: string;

  constructor(options: {
    region: Region,
    name: string,
    host: string,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }) {
    super(
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
}
/* ==== DESTACK_GENERATED_END:STRUCT:7601 ==== */