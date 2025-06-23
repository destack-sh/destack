import {
  Align,
  Axis2,
  Axis3,
  Border,
  Corners,
  Dimension,
  Direction,
  Distribute,
  Fill,
  Grid,
  GridSpan,
  Insets,
  IsSubject,
  Layout,
  MaterializationType,
  Node,
  NodeReference,
  Position,
  Script,
  Shadow,
  Space,
  Value,
  Vector2,
} from "@destack/language";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:TRAIT:10000 ==== */
export interface ContainerView {
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
  value: Map<string, Value>;
  readonly orderKey: string;
  name: string;
  position: Position | null;
  width: Dimension | null;
  height: Dimension | null;
  minWidth: Dimension | null;
  minHeight: Dimension | null;
  maxWidth: Dimension | null;
  maxHeight: Dimension | null;
  layout: Layout | null;
  direction: Direction | null;
  distribute: Distribute | null;
  align: Align | null;
  gap: Axis2 | null;
  padding: Insets | null;
  grid: Grid | null;
  gridSpan: GridSpan | null;
  aspectRatio: number | null;
  isWrap: boolean | null;
  isVisible: boolean | null;
  opacity: number | null;
  fill: Fill | null;
  rotation: Axis3 | null;
  skew: Vector2 | null;
  scale: number | null;
  shadow: Shadow | null;
  border: Border | null;
  radius: Corners | null;
  get script(): Script | null | null;
  set script(value: Script | null);
  scriptPtr: NodeReference | null;
}
/* ==== DESTACK_GENERATED_END:TRAIT:10000 ==== */
