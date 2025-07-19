import { EnumType } from "@destack/language/core/builtin/common";
import { registerEnumClass } from "@destack/language/registry";

/* ==== DESTACK_GENERATED_START:ENUM:30200 ==== */
/**
 * ConstraintType
 */
export enum ConstraintType {
  UNIQUE = 1,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.CONSTRAINT_TYPE, ConstraintType);
/* ==== DESTACK_GENERATED_END:ENUM:30200 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:30100 ==== */
/**
 * IndexType
 */
export enum IndexType {
  BTREE = 1,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.INDEX_TYPE, IndexType);
/* ==== DESTACK_GENERATED_END:ENUM:30100 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:701000 ==== */
/**
 * MethodType
 */
export enum MethodType {
  PROPERTY = 1,
  INSTANCE = 2,
  STATIC = 3,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.METHOD_TYPE, MethodType);
/* ==== DESTACK_GENERATED_END:ENUM:701000 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:701001 ==== */
/**
 * MethodCardinality
 */
export enum MethodCardinality {
  UNARY = 1,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.METHOD_CARDINALITY, MethodCardinality);
/* ==== DESTACK_GENERATED_END:ENUM:701001 ==== */
