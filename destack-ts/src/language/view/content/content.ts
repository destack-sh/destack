import {
  Align,
  Dimension,
  IsSubject,
  MaterializationType,
  Node,
  NodeReference,
  Position,
} from "@destack/language/core";
import { Script } from "@destack/language/logic";
import { Space } from "@destack/language/space";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:TRAIT:10200 ==== */
export interface ContentView {
  readonly id: string;
  get space(): Space | null | null;
  readonly spacePtr: NodeReference | null;
  readonly materialization: MaterializationType;
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): (Node & IsSubject) | null | null;
  readonly createdByPtr: NodeReference | null;
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): (Node & IsSubject) | null | null;
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
  align: Align | null;
  isVisible: boolean | null;
  opacity: number | null;
  get script(): Script | null | null;
  set script(value: Script | null);
  scriptPtr: NodeReference | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:10200 ==== */
