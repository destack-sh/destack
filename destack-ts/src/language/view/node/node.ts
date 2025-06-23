import { SplitView, CustomView, Dimension, MaterializationType, Canvas, NodeType, QueryConnection, StructType, Space, NodeReference, Graph, Script, User, Struct, FrameView, StructFrozen, PlaneShape, BuiltinObject, EnumType, Scene, Layer, Agent, Position, Window, activeSession, Node, Supergraph, AnnotationShape, CustomViewDefinition, ACTIVE_SESSION, LabelView, Session } from '@/language';

/* ==== DESTACK_GENERATED_START:TRAIT:10600 ==== */
export interface NodeView {
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
  readonly orderKey: string;
  name: string;
  position: Position | null;
  width: Dimension | null;
  height: Dimension | null;
  minWidth: Dimension | null;
  minHeight: Dimension | null;
  maxWidth: Dimension | null;
  maxHeight: Dimension | null;
  get script(): Script | null
  set script(value: Script | null);
  scriptPtr: NodeReference | null
}
/* ==== DESTACK_GENERATED_END:TRAIT:10600 ==== */