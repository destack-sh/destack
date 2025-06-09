import { ReadNodeGraph } from "@/language/core/graph";
import { generateNodeName } from "@/language/core/node";
import { makeType } from "@/language/core/type";
import { Transaction } from "@/language/core/transaction";
import { DestackType, ClassData, TypeKind } from "@/proto/wire";
import { FieldType, NodeType, ObjectType, TypeData } from "@/proto/wire/proto/lang";
import { toNodeRef } from "@/proto/wiring";

/** Create a Class. */
export function createClass(tx: Transaction, graph: ReadNodeGraph, options: { class: Partial<ClassData> }): ClassData {
  const siblings = graph.getChildren(options.class.parentPtr!, NodeType.CLASS);
  const name = options.class.name ?? generateNodeName({ metatype: ObjectType.CLASS, ...options.class }, siblings);
  const choice = tx.create({ metatype: NodeType.CLASS, ...options.class, name });
  return choice;
}

/** Get the Type of a Class. */
export function classToType(clazz: ClassData, of?: "instance" | "value", fieldTypes?: FieldType[]): TypeData {
  if (of == "instance") {
    return makeType({ kind: TypeKind.BASED_NODE, destackType: DestackType.FIELD, baseTypePtr: toNodeRef(clazz) });
  } else {
    return makeType({
      kind: TypeKind.CUSTOM_OBJECT,
      baseTypePtr: toNodeRef(clazz),
      baseFieldTypes: fieldTypes,
      propertyFieldTypes: fieldTypes,
    });
  }
}
