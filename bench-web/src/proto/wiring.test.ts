import { ObjectType } from "@/proto/wire";
import { fromRobustJson, toRobustJson } from "@/proto/wiring";
import { fabricate } from "@/system/graph.test";
import { OBJECT_TYPES } from "@/system/lang";
import { describe, expect, test } from "vitest";

describe("wiring", () => {
  const OBJECT_TYPES_NAMES = OBJECT_TYPES.map((t) => ObjectType[t]);
  test.each(OBJECT_TYPES_NAMES)("roundtrip robust json %s", (typeName) => {
    const objectType = ObjectType[typeName as any] as unknown as ObjectType;
    const object = fabricate(objectType);
    const packedJson = toRobustJson(object);
    const unpackedObject = fromRobustJson(packedJson);
    expect(unpackedObject).toEqual(object);
  });
});
