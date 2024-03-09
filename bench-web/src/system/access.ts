import { ReadType, type AccessType, type NodeReferenceData, EditType, UseType } from "@/proto/wire";

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
