import { Layer, Vector2, Agent, PlaneShape, Scene, Supergraph, StructFrozen, CustomViewDefinition, Border, EnumType, Distribute, Struct, SplitView, Session, Position, NodeReference, User, Axis2, Shadow, Script, MaterializationType, BuiltinObject, Graph, Grid, Direction, GridSpan, Dimension, CustomView, Align, NodeType, Node, StructType, Layout, Value, QueryConnection, Insets, LabelView, FrameView, Canvas, AnnotationShape, Corners, Window, Fill, Space, Axis3 } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:TRAIT:10000 ==== */
export interface ContainerView {
  readonly id: string;
  get space(): Space | null;
  spacePtr: NodeReference | null
  readonly materialization: MaterializationType;
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): Agent | User | null;
  createdByPtr: NodeReference | null
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): Agent | User | null;
  updatedByPtr: NodeReference | null
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
  get script(): Script | null
  set script(value: Script | null);
  scriptPtr: NodeReference | null
}
/* ==== DESTACK_GENERATED_END:TRAIT:10000 ==== */