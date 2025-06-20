import { CustomView, User, Scene, NodeReference, NumberInputView, PlaneShape, Supergraph, EnumType, CustomViewDefinition, SplitView, AnnotationShape, TextView, StructType, Space, Canvas, MaterializationType, Session, QueryConnection, ArrowShape, Layer, ThreadView, Theme, FrameView, Node, BuiltinObject, LineShape, Graph, Agent, NodeType, WizardView, Struct, StructFrozen, LabelView, SliderInputView } from '@/language';
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