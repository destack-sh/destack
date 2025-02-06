import { makeType } from "@/language/core/type";
import { BenchType, DatabaseData, FieldType, TypeData, TypeKind } from "@/proto/wire";
import { toNodeRef } from "@/proto/wiring";

/** Get the TypeData for a Database. */
export function databaseToType(
  database: DatabaseData,
  of: "instance" | "value" = "instance",
  fieldTypes?: FieldType[],
): TypeData | undefined {
  if (of == "instance") {
    return makeType({ kind: TypeKind.BASED_NODE, benchType: BenchType.RECORD, baseTypePtr: toNodeRef(database) });
  } else {
    fieldTypes = fieldTypes ?? [FieldType.MEMBER];
    return makeType({
      kind: TypeKind.CUSTOM_OBJECT,
      baseTypePtr: toNodeRef(database),
      baseFieldTypes: fieldTypes,
      propertyFieldTypes: fieldTypes,
    });
  }
}
