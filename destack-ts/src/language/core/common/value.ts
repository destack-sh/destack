import { EnumType, Node, QueryConnection, StructFrozen, StructType, NodeReference, ACTIVE_SESSION, Supergraph, Type, Struct, NodeType, Graph, BuiltinObject, Session, activeSession } from '@/language';
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
    super(
        // supergraph
        supergraph,
    );

    this.type = options.type;
    this.value = options.value;
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
/* ==== DESTACK_GENERATED_END:STRUCT:2500 ==== */