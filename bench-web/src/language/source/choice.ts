import { BenchType, TypeData, TypeKind } from "@/proto/wire";
import { makeType } from "@/language/core/type";
import { ChoiceData } from "@/proto/wire";
import { toNodeRef } from "@/proto/wiring";

/** Get the Type of a Choice. */
export function choiceToType(choice: ChoiceData): TypeData {
  return makeType({
    kind: TypeKind.BASED_NODE,
    benchType: BenchType.FIELD,
    baseTypePtr: toNodeRef(choice),
  });
}
