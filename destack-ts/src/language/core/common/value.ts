import { Graph, Supergraph, Struct, QueryConnection, NodeReference, activeSession, StructType, BuiltinObject, EnumType, ACTIVE_SESSION, NodeType, Session, Type, StructFrozen, Node } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:STRUCT:2500 ==== */
export class Value extends StructFrozen {
  readonly type: Type;
  readonly value: any;

  constructor(options: {
    type: Type,
    value: any,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }) {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(supergraph);
    this.type = options.type;
    this.value = options.value;
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