import { Session, CustomViewDefinition, CustomView, Canvas, Layer, User, NodeReference, BuiltinObject, ArrowShape, Agent, MaterializationType, PlaneShape, AnnotationShape, Scene, Graph, SliderInputView, WizardView, FrameView, LabelView, LineShape, TextView, Struct, Theme, ThreadView, Supergraph, NumberInputView, QueryConnection, SplitView, NodeType, Node, Space } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:TRAIT:12000 ==== */
export interface Style {
  readonly id: string;
  get space(): Space | null
  set space(value: Space | null);
  spacePtr: NodeReference | null
  materialization: MaterializationType;
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