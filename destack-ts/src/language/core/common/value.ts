import { BuiltinObject, Session, QueryConnection, NodeType, Supergraph, Node, Struct, Type, Graph, NodeReference } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:STRUCT:2500 ==== */
export class Value extends BuiltinObject {
  readonly type: Type;
  readonly value: any;

  constructor(
    type: Type,
    value: any,
    _supergraph: Supergraph
  ) {
    super(_supergraph);
    this.type = type;
    this.value = value;
  }


  static create(): Value {

    return new Value();
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