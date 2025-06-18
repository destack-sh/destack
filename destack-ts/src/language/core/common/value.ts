import { Graph, Type, Supergraph, StructFrozen, QueryConnection, StructType, NodeReference, NodeType, Session, Node, Struct, EnumType, BuiltinObject } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:STRUCT:2500 ==== */
export class Value extends StructFrozen {
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


  static create(options: {
    type: Type,
    value: any
  }): Value {

    return new Value(

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