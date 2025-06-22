import { Graph, Supergraph, Struct, QueryConnection, NodeReference, activeSession, StructType, BuiltinObject, EnumType, ACTIVE_SESSION, NodeType, Session, StructFrozen, Node, Region } from '@/language';
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
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(supergraph);
    this.region = options.region;
    this.name = options.name;
    this.host = options.host;
  }

  equals(other: any): boolean {
    throw new Error("Not implemented");
  }

  hash(): number {
    throw new Error("Not implemented");
  }

  validate(): void {
    throw new Error("Not implemented");
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:7601 ==== */