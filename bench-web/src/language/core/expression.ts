import { getPathKey } from "@/language/core/path";
import { getPropertyType } from "@/language/core/type";
import { packValue } from "@/language/core/value";
import { ExpressionType, ObjectType, PathData, type ExpressionData, type SortType } from "@/proto/wire";
import { computed, Ref } from "vue";

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
    valuePacked = packValue(options.value, propertyType, { wrapScalar: true });
  } else {
    valuePacked = options.valuePacked;
  }

  const expression: ExpressionData = {
    metatype: ObjectType.EXPRESSION,
    clauses: [],
    ...options,
    valuePacked,
  };
  if ("value" in expression) {
    delete expression.value;
  }
  return expression;
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
