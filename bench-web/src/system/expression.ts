import { ExpressionOp, ObjectType, type ExpressionData, type SortOp } from "@/proto/wire";
import { newStructId } from "@/proto/wiring";

export function makeExpression(options: { op: ExpressionOp } & Partial<ExpressionData>): ExpressionData {
  return {
    metatype: ObjectType.EXPRESSION,
    id: newStructId(),
    clauses: [],
    ...options,
  };
}

export function makeSort(options: Pick<ExpressionData, "sortMode" | "propertyPtr"> & { op: SortOp }): ExpressionData {
  return makeExpression({ ...(options as unknown as ExpressionData) });
}
