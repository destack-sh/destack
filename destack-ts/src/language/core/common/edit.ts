import {
  Agent,
  Field,
  Node,
  NodeReference,
  Origin,
  PropertyReference,
  Session,
  StructFrozen,
  StructType,
  Supergraph,
  User,
  Value,
} from "@/language";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:50050 ==== */
export enum EditType {
  CREATE = 1,
  UPSERT = 2,
  UPDATE = 3,
  MOVE = 4,
  ARCHIVE = 5,
  UNARCHIVE = 6,
  DELETE = 7,
  RESTORE = 8,
  ERASE = 9,
}
/* ==== DESTACK_GENERATED_END:ENUM:50050 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:50051 ==== */
export enum EditOperation {
  SET = 1,
  CLEAR = 2,
}
/* ==== DESTACK_GENERATED_END:ENUM:50051 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:50052 ==== */
export enum ChangeStatus {
  COMPLETED = 10,
  FAILED = 12,
  REJECTED = 13,
}
/* ==== DESTACK_GENERATED_END:ENUM:50052 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:50053 ==== */
export enum ChangeDebounce {
  LAZY = 10,
}
/* ==== DESTACK_GENERATED_END:ENUM:50053 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50020 ==== */
export class Edit extends StructFrozen {
  static metatype: StructType = StructType.EDIT;
  static __isFrozen__: boolean = true;

  readonly id: string;
  readonly type: EditType;
  readonly operation: EditOperation | null;
  get node(): Node {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr !== null) {
      if (this._supergraph === null) {
        return null;
      }
      return this._supergraph.get(nodePtr.id) as Node;
    }
    return null;
  }
  readonly nodePtr: NodeReference;
  readonly propPtr: PropertyReference | null;
  get field(): Field | null {
    const nodePtr: NodeReference | null = this.fieldPtr;
    if (nodePtr !== null) {
      if (this._supergraph === null) {
        return null;
      }
      return this._supergraph.get(nodePtr.id) as Field | null;
    }
    return null;
  }
  readonly fieldPtr: NodeReference | null;
  readonly key: Value | null;
  readonly value: Value | null;
  readonly undo: Edit | null;

  constructor(options: {
    id?: string;
    type: EditType;
    operation?: EditOperation | null;
    node: Node | NodeReference;
    propPtr?: PropertyReference | null;
    field?: Field | NodeReference | null;
    key?: Value | null;
    value?: Value | null;
    undo?: Edit | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
  }) {
    super(
      // supergraph
      options._supergraph ?? null,
    );

    this.id = options.id;
    this.type = options.type;
    this.operation = options.operation ?? null;
    this.nodePtr =
      options.node != null
        ? options.node.metatype == StructType.NODE_REFERENCE
          ? (options.node as NodeReference)
          : (options.node as Node).toRef()
        : null;
    this.propPtr = options.propPtr ?? null;
    this.fieldPtr =
      options.field != null
        ? options.field.metatype == StructType.NODE_REFERENCE
          ? (options.field as NodeReference)
          : (options.field as Node).toRef()
        : null;
    this.key = options.key ?? null;
    this.value = options.value ?? null;
    this.undo = options.undo ?? null;
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
/* ==== DESTACK_GENERATED_END:STRUCT:50020 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50021 ==== */
export class Change extends StructFrozen {
  static metatype: StructType = StructType.CHANGE;
  static __isFrozen__: boolean = true;

  readonly id: string;
  readonly name: string | null;
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): Agent | User | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr !== null) {
      if (this._supergraph === null) {
        return null;
      }
      return this._supergraph.get(nodePtr.id) as Agent | User | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference | null;
  readonly origin: Origin | null;
  readonly debounce: ChangeDebounce | null;
  readonly edits: Array<Edit>;

  constructor(options: {
    id?: string;
    name?: string | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: Agent | User | NodeReference | null;
    origin?: Origin | null;
    debounce?: ChangeDebounce | null;
    edits?: Array<Edit>;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
  }) {
    super(
      // supergraph
      options._supergraph ?? null,
    );

    this.id = options.id;
    this.name = options.name ?? null;
    this.createdAt = options.createdAt;
    this.createdByPtr =
      options.createdBy != null
        ? options.createdBy.metatype == StructType.NODE_REFERENCE
          ? (options.createdBy as NodeReference)
          : (options.createdBy as Node).toRef()
        : null;
    this.origin = options.origin ?? null;
    this.debounce = options.debounce ?? null;
    this.edits = options.edits ?? [];
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
/* ==== DESTACK_GENERATED_END:STRUCT:50021 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50022 ==== */
export class ChangeResult extends StructFrozen {
  static metatype: StructType = StructType.CHANGE_RESULT;
  static __isFrozen__: boolean = true;

  readonly id: string;
  readonly createdAt: Temporal.ZonedDateTime;
  readonly debounce: ChangeDebounce | null;
  readonly status: ChangeStatus;
  readonly edits: Array<Edit>;
  readonly cascadedEdits: Array<Edit>;

  constructor(options: {
    id?: string;
    createdAt?: Temporal.ZonedDateTime;
    debounce?: ChangeDebounce | null;
    status: ChangeStatus;
    edits?: Array<Edit>;
    cascadedEdits?: Array<Edit>;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
  }) {
    super(
      // supergraph
      options._supergraph ?? null,
    );

    this.id = options.id;
    this.createdAt = options.createdAt;
    this.debounce = options.debounce ?? null;
    this.status = options.status;
    this.edits = options.edits ?? [];
    this.cascadedEdits = options.cascadedEdits ?? [];
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
/* ==== DESTACK_GENERATED_END:STRUCT:50022 ==== */
