import { EnumType, Graph, StructFrozen, StructType, Supergraph, Struct, Session, BuiltinObject, QueryConnection, NodeType, Node, NodeReference } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:ENUM:8000 ==== */
export enum ModelDeveloper {
  OPENAI = 1010,
  ANTHROPIC = 1020,
  GOOGLE = 1030,
  XAI = 1060,
}
/* ==== DESTACK_GENERATED_END:ENUM:8000 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:8001 ==== */
export enum ModelProvider {
  OPENROUTER = 1000,
  OPENAI = 1010,
  ANTHROPIC = 1020,
  GOOGLE = 1030,
  XAI = 1040,
}
/* ==== DESTACK_GENERATED_END:ENUM:8001 ==== */