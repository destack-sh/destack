import { Graph, Supergraph, Struct, QueryConnection, NodeReference, activeSession, StructType, BuiltinObject, EnumType, ACTIVE_SESSION, NodeType, Session, StructFrozen, Node } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:ENUM:50041 ==== */
export enum ErrorType {
  ABORTED = 2,
  RUNTIME_UNAVAILABLE = 3,
  RUN_IMPOSSIBLE = 4,
  NOT_SUPPORTED = 5,
  INVALID_VALUE = 10,
  INVALID_COMPUTED = 11,
  CODE_INVALID = 20,
  TEXT_INVALID = 21,
  INCAPABLE = 100,
  REFUSED = 101,
  NON_RETRYABLE = 499,
  INTERRUPTION_CANCELLED = 500,
  MODEL_FAILED = 501,
}
/* ==== DESTACK_GENERATED_END:ENUM:50041 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:4001 ==== */
export class Error extends StructFrozen {
  readonly type: ErrorType;
  readonly title: string | null;
  readonly text: string | null;

  constructor(options: {
    type: ErrorType,
    title?: string | null,
    text?: string | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }) {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(supergraph);
    this.type = options.type;
    this.title = options.title ?? null;
    this.text = options.text ?? null;
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
/* ==== DESTACK_GENERATED_END:STRUCT:4001 ==== */