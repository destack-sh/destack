import { NumberInputView, ArrowShape, EnumType, StructType, CustomView, Scene, Theme, CustomViewDefinition, Node, Canvas, QueryConnection, Layer, User, NodeReference, Graph, LabelView, SplitView, Agent, Space, StructFrozen, ThreadView, Struct, LineShape, BuiltinObject, AnnotationShape, SliderInputView, NodeType, TextView, Session, MaterializationType, FrameView, PlaneShape, Supergraph, WizardView } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:TRAIT:12000 ==== */
export interface Style {
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
}
/* ==== DESTACK_GENERATED_END:TRAIT:12000 ==== */