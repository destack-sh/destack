import { EnumType, Node, QueryConnection, StructFrozen, StructType, NodeReference, ACTIVE_SESSION, Supergraph, Struct, NodeType, Region, Graph, BuiltinObject, Session, activeSession } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

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
        supergraph,
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