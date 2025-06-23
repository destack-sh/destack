import { Session, StructFrozen, StructType, Supergraph, Type } from "@/language";

/* ==== DESTACK_GENERATED_START:STRUCT:2500 ==== */
export class Value extends StructFrozen {
  static metatype: StructType = StructType.VALUE;
  static __isFrozen__: boolean = true;

  readonly type: Type;
  readonly value: any;

  constructor(options: { type: Type; value: any; _session?: Session | null; _supergraph?: Supergraph | null }) {
    super(
      // supergraph
      options._supergraph ?? null,
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
