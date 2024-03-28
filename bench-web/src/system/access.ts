import { ReadType, type AccessType, type NodeReferenceData, EditType, UseType, AccessMatrixData } from "@/proto/wire";
import { toRef, type MaybeRef, type Ref } from "vue";

export const READ_TYPES: ReadType[] = Object.keys(ReadType)
  .map((key) => Number(key))
  .filter((key) => !isNaN(key));
export const EDIT_TYPES: EditType[] = Object.keys(EditType)
  .map((key) => Number(key))
  .filter((key) => !isNaN(key));
export const USE_TYPES: UseType[] = Object.keys(UseType)
  .map((key) => Number(key))
  .filter((key) => !isNaN(key));

export type AccessArbiter = {
  can(access: AccessType, node: NodeReferenceData): boolean;
};

export function accessFull(): AccessArbiter {
  return {
    can(access: AccessType, node: NodeReferenceData) {
      return true;
    },
  };
}

export function accessNone(): AccessArbiter {
  return {
    can(access: AccessType, node: NodeReferenceData) {
      return false;
    },
  };
}

export function accessFromMatrix(matrix: AccessMatrixData) {
  return {
    can(access: AccessType, node: NodeReferenceData) {
      // TODO :Broken!: parse & watch access
      return true;
    },
  };
}

export class AccessProxy {
  access: Ref<AccessArbiter | null>;
  readonly defaultAccess: AccessArbiter;

  constructor(arbiter: MaybeRef<AccessArbiter | null>, options: { default: AccessArbiter }) {
    this.access = toRef(arbiter);
    this.defaultAccess = options.default;
  }

  can(access: AccessType, node: NodeReferenceData) {
    if (this.access.value != null) return this.access.value.can(access, node);
    else return this.defaultAccess.can(access, node);
  }
}
