import { PlaneShape, Agent, BuiltinObject, ACTIVE_SESSION, Session, Scene, StructFrozen, Graph, Struct, AnnotationShape, activeSession, StructType, NodeType, Window, Canvas, FrameView, Dimension, Supergraph, QueryConnection, NodeReference, MaterializationType, User, Position, CustomView, LabelView, Space, CustomViewDefinition, SplitView, Script, EnumType, Layer, Node } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:TRAIT:10400 ==== */
export interface InputView {
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
  isVisible: boolean | null;
  opacity: number | null;
  get script(): Script | null
  set script(value: Script | null);
  scriptPtr: NodeReference | null
}
/* ==== DESTACK_GENERATED_END:TRAIT:10400 ==== */