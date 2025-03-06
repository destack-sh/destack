import { ReadNodeGraph } from "@/language/core/graph";
import { generateNodeName } from "@/language/core/node";
import { makeType } from "@/language/core/type";
import { Transaction } from "@/language/runtime/transaction";
import { BenchType, DatabaseData, FieldType, NodeType, ObjectType, TypeData, TypeKind } from "@/proto/wire";
import { toNodeRef } from "@/proto/wiring";

/** Create a Database. */
export function createDatabase(
  tx: Transaction,
  graph: ReadNodeGraph,
  options: { database: Partial<DatabaseData> },
): DatabaseData {
  if (options.database.definitionPtr == null) {
    throw new Error("cannot create inline Database without block");
  }
  const siblings = graph.getChildren(options.database.parentPtr!, NodeType.DATABASE);
  const name =
    options.database.name ?? generateNodeName({ metatype: ObjectType.DATABASE, ...options.database }, siblings);
  const database = tx.create({ metatype: NodeType.DATABASE, ...options.database, name });
  return database;
}

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
