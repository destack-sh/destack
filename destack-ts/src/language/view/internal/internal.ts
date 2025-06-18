import { PlaneShape, LabelView, Graph, Position, MaterializationType, StructFrozen, CustomViewDefinition, Struct, EnumType, Script, QueryConnection, NodeReference, SplitView, Node, Scene, Layer, CustomView, FrameView, Dimension, AnnotationShape, NodeType, Session, Agent, Space, User, Window, StructType, Canvas, Supergraph, BuiltinObject } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:TRAIT:10650 ==== */
export interface InternalView {
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
/* ==== DESTACK_GENERATED_END:TRAIT:10650 ==== */