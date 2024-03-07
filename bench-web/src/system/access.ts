import type { AccessType, NodeReferenceData } from "@/proto/wire"


export type AccessQuery = {
  can(access: AccessType, node: NodeReferenceData): boolean;
}