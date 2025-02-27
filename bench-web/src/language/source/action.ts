import { ACTION_TYPES } from "@/language/core/const";
import { makeType, makeTypeConstraint } from "@/language/core/type";

import { ActionCategory, ActionData, ActionType, BenchType, FieldType, TypeData, TypeKind } from "@/proto/wire";
import { toNodeRef } from "@/proto/wiring";
import { assertNever } from "@/utils/functools";

// NOTE: the default ActionCategory is WORK
export const ACTION_CATEGORY_BY_TYPE: Partial<Record<ActionType, ActionCategory>> = {
  [ActionType.THINK]: ActionCategory.THINK,
  [ActionType.WAIT]: ActionCategory.WAIT,
  [ActionType.SEND]: ActionCategory.TYPE,
  [ActionType.RECEIVE]: ActionCategory.TYPE,
  [ActionType.YIELD]: ActionCategory.WAIT,
};
for (const type of ACTION_TYPES) {
  if (type >= 1000 && type < 1100) {
    ACTION_CATEGORY_BY_TYPE[type] = ActionCategory.INTERACT;
  } else if (type >= 1100 && type < 1200) {
    ACTION_CATEGORY_BY_TYPE[type] = ActionCategory.READ;
  } else if (type >= 1200 && type < 1300) {
    ACTION_CATEGORY_BY_TYPE[type] = ActionCategory.BROWSE;
  }
}

export function getActionCategory(action: ActionData | ActionType): ActionCategory {
  if (typeof action == "number") {
    return ACTION_CATEGORY_BY_TYPE[action] ?? ActionCategory.WORK;
  } else {
    return ACTION_CATEGORY_BY_TYPE[action.type] ?? ActionCategory.WORK;
  }
}

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
    } else if (action.type == ActionType.COMPLETE) {
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
