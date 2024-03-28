import { ReadType, type AccessType, type NodeReferenceData, EditType, UseType, AccessMatrixData } from "@/proto/wire";

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

export function accessAsOwner(): AccessArbiter {
  return {
    can(access: AccessType, node: NodeReferenceData) {
      return true;
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
  arbiter: AccessArbiter | null;
  readonly defaultArbiter: AccessArbiter;

  constructor(arbiter: AccessArbiter | null, defaultArbiter: AccessArbiter) {
    this.arbiter = arbiter;
    this.defaultArbiter = defaultArbiter;
  }

  can(access: AccessType, node: NodeReferenceData) {
    if (this.arbiter != null) return this.arbiter.can(access, node);
    else return this.defaultArbiter.can(access, node);
  }
}
