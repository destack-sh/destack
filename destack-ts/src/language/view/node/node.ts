import { Agent, Dimension, MaterializationType, NodeReference, Position, Script, Space, User } from "@/language";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:TRAIT:10600 ==== */
export interface NodeView {
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
  position: Position | null;
  width: Dimension | null;
  height: Dimension | null;
  minWidth: Dimension | null;
  minHeight: Dimension | null;
  maxWidth: Dimension | null;
  maxHeight: Dimension | null;
  get script(): Script | null;
  set script(value: Script | null);
  scriptPtr: NodeReference | null;
} /* ==== DESTACK_GENERATED_END:TRAIT:10600 ==== */
