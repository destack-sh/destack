import { SplitView, CustomView, ThreadView, MaterializationType, Canvas, NodeType, QueryConnection, StructType, Space, NodeReference, Graph, NumberInputView, User, Struct, FrameView, StructFrozen, PlaneShape, BuiltinObject, EnumType, LineShape, Scene, Layer, Agent, SliderInputView, TextView, Theme, activeSession, Node, Supergraph, WizardView, AnnotationShape, CustomViewDefinition, ACTIVE_SESSION, LabelView, Session, ArrowShape } from '@/language';

/* ==== DESTACK_GENERATED_START:TRAIT:12000 ==== */
export interface Style {
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
}
/* ==== DESTACK_GENERATED_END:TRAIT:12000 ==== */