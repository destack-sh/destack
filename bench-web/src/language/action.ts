import { makeType, makeTypeConstraint } from "@/language/field";

import { BenchType, FieldType, TypeKind } from "@/proto/wire";
import { ActionData, ActionType, TypeData } from "@/proto/wire";
import { toNodeRef } from "@/proto/wiring";
import { assertNever } from "@/utils/functools";

/** Get the Type for an Action */
export function actionToType(action: ActionData, of: "instance" | "value", fieldTypes: FieldType[]): TypeData {
  if (of == "instance") {
    return makeType({ kind: TypeKind.BASED_NODE, benchType: BenchType.RUN, baseTypePtr: toNodeRef(action) });
  } else if (of == "value") {
    if (action.type == ActionType.START) {
      return makeType({
        kind: TypeKind.CUSTOM_OBJECT,
        baseTypePtr: action.parentPtr,
        baseFieldTypes: [FieldType.INPUT],
      });
    } else if (action.type == ActionType.COMPLETE) {
      return makeType({
        kind: TypeKind.CUSTOM_OBJECT,
        baseTypePtr: action.parentPtr,
        baseFieldTypes: [FieldType.OUTPUT],
      });
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
