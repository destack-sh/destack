import { getPropertyType } from "@/language/field";
import { packValue } from "@/language/value";
import {
  BlockData,
  ExpressionType,
  ObjectType,
  RunData,
  ActionData,
  UserData,
  type ExpressionData,
  type SortType
} from "@/proto/wire";

export function makeExpression(
  options: { type: ExpressionType; value?: any } & Partial<ExpressionData>,
): ExpressionData {
  let valuePacked;
  if (options.value != null) {
    if (options.propertyPtr == null) throw new Error("propertyPtr required to pack Expression.value");
    let propertyType = getPropertyType(options.propertyPtr);
    if (options.type == ExpressionType.IN || options.type == ExpressionType.NOT_IN) {
      propertyType = { ...propertyType, isList: true }; // coerce type to list
    }
    valuePacked = packValue(options.value, propertyType);
  } else {
    valuePacked = options.valuePacked;
  }

  return {
    metatype: ObjectType.EXPRESSION,
    clauses: [],
    ...options,
    valuePacked,
  };
}

export function makeSort(
  options: Pick<ExpressionData, "sortMode" | "propertyPtr"> & { type: SortType },
): ExpressionData {
  return makeExpression({ ...(options as unknown as ExpressionData) });
}

export function makeAndConditional(clauses: ExpressionData[]): ExpressionData | undefined {
  if (clauses.length == 0) return undefined;
  else return makeExpression({ type: ExpressionType.AND, clauses });
}

export type EditSubject = UserData | RunData | BlockData | ActionData;

