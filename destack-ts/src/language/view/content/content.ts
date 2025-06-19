import { Script, Canvas, PlaneShape, EnumType, StructFrozen, Align, CustomViewDefinition, Agent, FrameView, StructType, QueryConnection, NodeType, Supergraph, Session, AnnotationShape, Node, LabelView, Dimension, Struct, MaterializationType, SplitView, Graph, User, Scene, Position, Window, CustomView, BuiltinObject, Layer, NodeReference, Space } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:TRAIT:10200 ==== */
export interface ContentView {
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
  align: Align | null;
  isVisible: boolean | null;
  opacity: number | null;
  get script(): Script | null
  set script(value: Script | null);
  scriptPtr: NodeReference | null
}
/* ==== DESTACK_GENERATED_END:TRAIT:10200 ==== */