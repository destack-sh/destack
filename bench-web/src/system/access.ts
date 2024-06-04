import {
  ReadType,
  type AccessType,
  type NodeReferenceData,
  EditType,
  UseType,
  AccessMatrixData,
  NodeType,
} from "@/proto/wire";
import { toRef, type MaybeRef, type Ref, ref } from "vue";

export const READ_TYPES: ReadType[] = Object.keys(ReadType)
  .map((key) => Number(key))
  .filter((key) => !isNaN(key));
export const EDIT_TYPES: EditType[] = Object.keys(EditType)
  .map((key) => Number(key))
  .filter((key) => !isNaN(key));
export const USE_TYPES: UseType[] = Object.keys(UseType)
  .map((key) => Number(key))
  .filter((key) => !isNaN(key));

type AccessOptions = { scope?: NodeReferenceData };
type SomeAccessType = AccessType | ReadType | EditType | UseType;

export type AccessArbiter = {
  can(verb: SomeAccessType, node: NodeType, options?: AccessOptions): boolean;
  canRef(verb: SomeAccessType, node: MaybeRef<NodeType>, options?: MaybeRef<AccessOptions>): Ref<boolean>;
};

export function accessFull(): AccessArbiter {
  return {
    can: () => true,
    canRef: () => ref(true),
  };
}

export function accessNone(): AccessArbiter {
  return {
    can: () => false,
    canRef: () => ref(false),
  };
}

export function accessFromMatrix(matrix: AccessMatrixData) {
  // TODO :Incomplete!: parse & watch access
  return {
    can(verb: SomeAccessType, node: NodeType, options?: AccessOptions) {
      return true;
    },
    canRef(verb: SomeAccessType, node: MaybeRef<NodeType>, options?: MaybeRef<AccessOptions>) {
      return ref(true);
    },
  };
}

/** Shallow reactive proxy on top of a deferred access arbiter. */
export class AccessProxy {
  access: Ref<AccessArbiter | null>;
  readonly defaultAccess: AccessArbiter;

  constructor(arbiter: MaybeRef<AccessArbiter | null>, options: { default: AccessArbiter }) {
    this.access = toRef(arbiter);
    this.defaultAccess = options.default;
  }

  can(verb: SomeAccessType, node: NodeType, options?: AccessOptions) {
    if (this.access.value != null) return this.access.value.can(verb, node);
    else return this.defaultAccess.can(verb, node);
  }

  canRef(verb: SomeAccessType, node: MaybeRef<NodeType>, options?: MaybeRef<AccessOptions>) {
    if (this.access.value != null) return this.access.value.canRef(verb, node);
    else return this.defaultAccess.canRef(verb, node);
  }
}
