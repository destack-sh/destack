import {
  BlockData,
  ExpressionOp,
  NodeReferenceData,
  NodeType,
  ObjectType,
  RunData,
  StepData,
  UserData,
  type ExpressionData,
  type SortOp,
} from "@/proto/wire";
import { supergraph } from "@/system/connection";
import { getPropertyType, packValueSimpleStruct } from "@/language/value";

export function makeExpression(options: { op: ExpressionOp; value?: any } & Partial<ExpressionData>): ExpressionData {
  let valuePacked;
  if (options.value != null) {
    if (options.propertyPtr == null) throw new Error("propertyPtr required to pack Expression.value");
    let propertyType = getPropertyType(options.propertyPtr);
    if (options.op == ExpressionOp.IN || options.op == ExpressionOp.NOT_IN) {
      propertyType = { ...propertyType, isList: true }; // coerce type to list
    }
    valuePacked = packValueSimpleStruct(options.value, propertyType);
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

export function makeSort(options: Pick<ExpressionData, "sortMode" | "propertyPtr"> & { op: SortOp }): ExpressionData {
  return makeExpression({ ...(options as unknown as ExpressionData) });
}

export type EditSubject = UserData | RunData | BlockData | StepData;

/** Resolves an arbitrary subject in the supergraph */
export function resolveSubject(subjectPtr: NodeReferenceData | null | undefined): EditSubject | null {
  let subject: EditSubject | null;
  if (subjectPtr != null) {
    if (subjectPtr.type == NodeType.RUN) {
      subject = supergraph.get({ ck: subjectPtr.baseCk }) as BlockData | StepData | null;
    } else {
      subject = supergraph.get(subjectPtr) as UserData | null;
    }
  } else {
    subject = null;
  }
  return subject;
}
