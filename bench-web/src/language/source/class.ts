import { TypeKind } from "@/proto/wire";
import { BenchType } from "@/proto/wire";
import { ClassData } from "@/proto/wire";
import { FieldType, TypeData } from "@/proto/wire/proto/lang";
import { makeType } from "@/language/core/type";
import { toNodeRef } from "@/proto/wiring";

/** Get the TypeData for a class. */
export function classToType(clazz: ClassData, of: "instance" | "value", fieldTypes: FieldType[]): TypeData {
  if (of == "instance") {
    return makeType({ kind: TypeKind.BASED_NODE, benchType: BenchType.FIELD, baseTypePtr: toNodeRef(clazz) });
  } else {
    return makeType({
      kind: TypeKind.CUSTOM_OBJECT,
      baseTypePtr: toNodeRef(clazz),
      baseFieldTypes: fieldTypes,
      propertyFieldTypes: fieldTypes,
    });
  }
}
