import { Agent, MaterializationType, NodeReference, Space, User } from "@/language";

/* ==== DESTACK_GENERATED_START:TRAIT:12000 ==== */
export interface Style {
  readonly id: string;
  get space(): Space | null;
  readonly spacePtr: NodeReference | null;
  readonly materialization: MaterializationType;
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): Agent | User | null;
  readonly createdByPtr: NodeReference | null;
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): Agent | User | null;
  readonly updatedByPtr: NodeReference | null;
  readonly deletedAt: Temporal.ZonedDateTime | null;
  readonly orderKey: string;
  name: string;
}
/* ==== DESTACK_GENERATED_END:TRAIT:12000 ==== */
