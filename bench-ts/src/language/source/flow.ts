import { ReadNodeGraph } from "@/language/core/graph";
import { generateNodeName } from "@/language/core/node";
import { makeType } from "@/language/core/type";
import { Transaction } from "@/language/core/transaction";
import {
  ActionType,
  BenchType,
  FieldType,
  FlowData,
  FlowType,
  NodeType,
  ObjectType,
  TypeData,
  TypeKind,
} from "@/proto/wire";
import { toNodeRef } from "@/proto/wiring";

/** Create a Flow. */
export function createFlow(tx: Transaction, graph: ReadNodeGraph, options: { flow: Partial<FlowData> }): FlowData {
  if (options.flow.definitionPtr == null) {
    throw new Error("cannot create inline Flow without block");
  }

  // create
  const siblings = graph.getChildren(options.flow.parentPtr!, NodeType.FLOW);
  const name = options.flow.name ?? generateNodeName({ metatype: ObjectType.FLOW, ...options.flow }, siblings);
  const flow = tx.create({ metatype: NodeType.FLOW, type: FlowType.ACTION, ...options.flow, name });

  // create default contents
  const start = tx.create({
    metatype: NodeType.ACTION,
    type: ActionType.START,
    name: "Start",
    parentPtr: toNodeRef(flow),
    packagePtr: flow.packagePtr,
  });

  return flow;
}

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
