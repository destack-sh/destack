import { fabricate } from "@/language/graph.test";
import { OBJECT_TYPES, getTkFromPtrMaybe } from "@/language/const";
import { BenchType, NodeType, ObjectType, PrimitiveType, TypeKind } from "@/proto/wire";
import { describe, expect, test } from "vitest";
import { type TypeIdentity, encodeTypeIdentity, decodeTypeIdentity } from "@/language/field";
import { packBuiltinObject, unpackBuiltinObject } from "@/language/value";

// the test data & targets are from the backend bench implementation
const TEST_TYPE_IDENTITIES: (Partial<TypeIdentity> & { kind: TypeKind; identityKey: string })[] = [
  {
    kind: TypeKind.PRIMITIVE,
    primitiveType: PrimitiveType.DATETIME,
    isSecret: false,
    isList: false,
    identityKey: "pe",
  },
  { kind: TypeKind.NODE, benchType: BenchType.USER, isSecret: false, isList: true, identityKey: "N" },
  { kind: TypeKind.BASED_NODE, benchType: BenchType.FIELD, isSecret: false, isList: true, identityKey: "N" },
  { kind: TypeKind.STRUCT, benchType: BenchType.TEXT, isSecret: false, isList: false, identityKey: "sg7C" },
  { kind: TypeKind.ENUM, benchType: BenchType.OBJECT_TYPE, isSecret: true, isList: false, identityKey: "!ek4E" },
  {
    kind: TypeKind.OBJECT,
    baseTypePtr: {
      metatype: ObjectType.NODE_REFERENCE,
      nodeType: NodeType.BLOCK,
      ck: "12345678-ffff-0000-0000-000000000000",
    },
    isSecret: false,
    isList: false,
    identityKey: "oEjRWeP//AAA=",
  },
];

describe("type encoding", () => {
  test.each(TEST_TYPE_IDENTITIES)("encode and decode type identity", (target) => {
    const identityKey = encodeTypeIdentity({ isRequired: false, isSecret: false, isList: false, ...target });
    expect(identityKey).toBe(target.identityKey);

    const decoded = decodeTypeIdentity(identityKey);
    expect(decoded.primitiveType).toBe(target.primitiveType);
    expect(decoded.isList).toBe(target.isList);
    expect(decoded.isSecret).toBe(target.isSecret);
  });
});

describe("packing structs", () => {
  const OBJECT_TYPES_NAMES = OBJECT_TYPES.map((t) => ObjectType[t]);
  test.each(OBJECT_TYPES_NAMES)("roundtrip struct value %s", (typeName) => {
    const objectType = ObjectType[typeName as any] as unknown as ObjectType;
    const value = fabricate(objectType);
    const valuePacked = packBuiltinObject(value);
    const valueUnpacked = unpackBuiltinObject(valuePacked);
    expect(valueUnpacked).toEqual(value);
  });
});
