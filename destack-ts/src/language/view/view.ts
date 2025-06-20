import { CustomView, User, Scene, NodeReference, Window, Dimension, PlaneShape, Supergraph, EnumType, CustomViewDefinition, SplitView, AnnotationShape, StructType, Space, Canvas, Script, MaterializationType, Session, QueryConnection, Position, Layer, FrameView, Node, BuiltinObject, Graph, Agent, NodeType, Struct, StructFrozen, LabelView } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:TRAIT:9001 ==== */
export interface View {
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
/* ==== DESTACK_GENERATED_END:TRAIT:9001 ==== */