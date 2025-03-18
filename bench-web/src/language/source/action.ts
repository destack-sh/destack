import { makeType, makeTypeConstraint } from "@/language/core/type";

import { ActionData, ActionType, BenchType, FieldType, TypeData, TypeKind } from "@/proto/wire";
import { toNodeRef } from "@/proto/wiring";
import { assertNever } from "@/utils/functools";

/** Get the Type for an Action */
export function actionToType(
  action: ActionData,
  of: "instance" | "value",
  fieldTypes: FieldType[],
): TypeData | undefined {
  if (of == "instance") {
    return makeType({ kind: TypeKind.BASED_NODE, benchType: BenchType.RUN, baseTypePtr: toNodeRef(action) });
  } else if (of == "value") {
    if (action.type == ActionType.START) {
      if (action.parentPtr != null && (!fieldTypes.length || fieldTypes.includes(FieldType.OUTPUT))) {
        return makeType({
          kind: TypeKind.CUSTOM_OBJECT,
          baseTypePtr: action.parentPtr,
          baseFieldTypes: [FieldType.INPUT],
        });
      } else {
        return undefined;
      }
    } else if (action.type == ActionType.END) {
      if (action.parentPtr != null && (!fieldTypes.length || fieldTypes.includes(FieldType.INPUT))) {
        return makeType({
          kind: TypeKind.CUSTOM_OBJECT,
          baseTypePtr: action.parentPtr,
          baseFieldTypes: [FieldType.OUTPUT],
        });
      } else {
        return undefined;
      }
    } else {
      if (fieldTypes.includes(FieldType.INPUT) || fieldTypes.includes(FieldType.OUTPUT)) {
        const basePtr = action.type == ActionType.TOOL ? action.toolPtr : toNodeRef(action);
        return makeType({
          kind: TypeKind.PARTIAL_OBJECT,
          baseTypePtr: basePtr,
          benchType: BenchType.ACTION,
          baseFieldTypes: fieldTypes,
          propertyFieldTypes: fieldTypes,
          constraint: makeTypeConstraint({ nodeSubtypes: [action.type] }),
        });
      } else {
        return makeType({
          kind: TypeKind.CUSTOM_OBJECT,
          baseTypePtr: toNodeRef(action),
          baseFieldTypes: fieldTypes,
        });
      }
    }
  } else {
    assertNever(of);
  }
}
