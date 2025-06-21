import { Struct, NodeType, Session, NodeReference, Supergraph, Node, StructType, StructFrozen, QueryConnection, BuiltinObject, EnumType, Graph, Region } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:STRUCT:7601 ==== */
export class GalaxyInfo extends Struct {
  region: Region;
  name: string;
  host: string;

  constructor(
    region: Region,
    name: string,
    host: string,
    _supergraph: Supergraph | null
  ) {
    super(_supergraph);
    this.region = region;
    this.name = name;
    this.host = host;
  }


  static create(options: {
    region: Region,
    name: string,
    host: string,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }): GalaxyInfo {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new GalaxyInfo(
      options.region,
      options.name,
      options.host,
      supergraph
    );
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