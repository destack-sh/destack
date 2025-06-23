import { SplitView, Border, CustomView, Dimension, MaterializationType, Canvas, NodeType, QueryConnection, Shadow, StructType, Space, NodeReference, Axis3, Graph, Script, GridSpan, Align, Value, Vector2, Insets, User, Struct, FrameView, StructFrozen, PlaneShape, Direction, BuiltinObject, EnumType, Corners, Layout, Scene, Layer, Axis2, Agent, Position, Fill, Grid, Window, activeSession, Node, Supergraph, AnnotationShape, CustomViewDefinition, ACTIVE_SESSION, Distribute, LabelView, Session } from '@/language';

/* ==== DESTACK_GENERATED_START:TRAIT:10000 ==== */
export interface ContainerView {
  readonly id: string;
  get space(): Space | null;
  readonly spacePtr: NodeReference | null
  readonly materialization: MaterializationType;
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): Agent | User | null;
  readonly createdByPtr: NodeReference | null
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): Agent | User | null;
  readonly updatedByPtr: NodeReference | null
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