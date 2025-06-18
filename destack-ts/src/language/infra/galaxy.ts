import { Graph, Supergraph, Region, StructFrozen, QueryConnection, StructType, NodeReference, NodeType, Session, Node, Struct, EnumType, BuiltinObject } from '@/language';
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
    _supergraph: Supergraph
  ) {
    super(_supergraph);
    this.region = region;
    this.name = name;
    this.host = host;
  }


  static create(options: {
    region: Region,
    name: string,
    host: string
  }): GalaxyInfo {

    return new GalaxyInfo(

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