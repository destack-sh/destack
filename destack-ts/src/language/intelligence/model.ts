import { EnumType } from "@destack/language/core";
import { registerEnumClass } from "@destack/language/registry";

/* ==== DESTACK_GENERATED_START:ENUM:120000 ==== */
/**
 * ModelDeveloper
 */
export enum ModelDeveloper {
  OPENAI = 1010,
  ANTHROPIC = 1020,
  GOOGLE = 1030,
  XAI = 1060,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.MODEL_DEVELOPER, ModelDeveloper);
/* ==== DESTACK_GENERATED_END:ENUM:120000 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:120001 ==== */
/**
 * ModelProvider
 */
export enum ModelProvider {
  OPENROUTER = 1000,
  OPENAI = 1010,
  ANTHROPIC = 1020,
  GOOGLE = 1030,
  XAI = 1040,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.MODEL_PROVIDER, ModelProvider);
/* ==== DESTACK_GENERATED_END:ENUM:120001 ==== */
