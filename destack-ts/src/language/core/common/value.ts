import { EnumType, Graph, StructFrozen, StructType, Supergraph, Type, Struct, Session, BuiltinObject, QueryConnection, NodeType, Node, NodeReference } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:STRUCT:2500 ==== */
export class Value extends StructFrozen {
  readonly type: Type;
  readonly value: any;

  constructor(
    type: Type,
    value: any,
    _supergraph: Supergraph | null
  ) {
    super(_supergraph);
    this.type = type;
    this.value = value;
  }


  static create(options: {
    type: Type,
    value: any,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }): Value {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new Value(
      options.type,
      options.value,
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
/* ==== DESTACK_GENERATED_END:STRUCT:2500 ==== */