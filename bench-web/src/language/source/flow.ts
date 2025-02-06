import { makeType } from "@/language/core/type";
import { BenchType, FieldType, FlowData, TypeData, TypeKind } from "@/proto/wire";
import { toNodeRef } from "@/proto/wiring";

/** Get the Type of a Flow. */
export function flowToType(flow: FlowData, of: "instance" | "value" = "instance", fieldTypes?: FieldType[]): TypeData {
  if (of == "instance") {
    return makeType({ kind: TypeKind.BASED_NODE, benchType: BenchType.RUN, baseTypePtr: toNodeRef(flow) });
  } else {
    fieldTypes = fieldTypes ?? [];
    return makeType({
      kind: TypeKind.CUSTOM_OBJECT,
      baseTypePtr: toNodeRef(flow),
      baseFieldTypes: fieldTypes,
      propertyFieldTypes: fieldTypes,
    });
  }
}
