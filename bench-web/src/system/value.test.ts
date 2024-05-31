import { BenchType, EnumType, NodeType, ObjectType, PrimitiveType, StructType, TypeKind } from "@/proto/wire";
import { fabricate } from "@/system/graph.test";
import { OBJECT_TYPES, getTkFromPtrMaybe } from "@/system/lang";
import {
  _packStructValueScalar,
  _unpackStructValueScalar,
  decodeTypeIdentity,
  encodeTypeIdentity,
  type TypeIdentity,
} from "@/system/value";
import { describe, expect, test } from "vitest";

// TYPE_IDENTITIES: tuple[tuple[TypeInfo, str], ...] = (
// 	(TypeInfo(primitive_type=PrimitiveType.DATETIME), "pU"),
// 	(TypeInfo(bench_type=NodeType.USER, is_list=True), "NdD"),
// 	(TypeInfo(bench_type=StructType.TEXT), "sIS"),
// 	(TypeInfo(bench_type=EnumType.OBJECT_TYPE, is_secret=True), "!eUf"),
// 	(
// 			TypeInfo(
// 					bench_type=NodeType.FIELD,
// 					base_type_ptr=NodeReference(
// 							type=NodeType.BLOCK, ck=UUID("12345678-ffff-0000-0000-000000000000")
// 					),
// 					is_secret=True,
// 					is_list=True,
// 			),
// 			"!BEjRWeP//AAA=g",
// 	),
// 	(
// 			TypeInfo(
// 					base_type_ptr=NodeReference(
// 							type=NodeType.BLOCK, ck=UUID("12345678-ffff-0000-0000-000000000000")
// 					),
// 			),
// 			"aEjRWeP//AAA=",
// 	),
// )

// the test data & targets are from the backend bench implementation
const TEST_TYPE_IDENTITIES: (Partial<TypeIdentity> & { identityKey: string })[] = [
  {
    kind: TypeKind.PRIMITIVE,
    primitiveType: PrimitiveType.DATETIME,
    isSecret: false,
    isList: false,
    identityKey: "pe",
  },
  { kind: TypeKind.NODE, benchType: BenchType.USER, isSecret: false, isList: true, identityKey: "NdD" },
  { kind: TypeKind.STRUCT, benchType: BenchType.TEXT, isSecret: false, isList: false, identityKey: "sIS" },
  { kind: TypeKind.ENUM, benchType: BenchType.OBJECT_TYPE, isSecret: true, isList: false, identityKey: "!eUf" },
  {
    kind: TypeKind.BASED_NODE,
    benchType: BenchType.FIELD,
    baseTypePtr: {
      metatype: ObjectType.NODE_REFERENCE,
      type: NodeType.BLOCK,
      ck: "12345678-ffff-0000-0000-000000000000",
    },
    isSecret: true,
    isList: true,
    identityKey: "!BEjRWeP//AAA=g",
  },
  {
    kind: TypeKind.OBJECT,
    baseTypePtr: {
      metatype: ObjectType.NODE_REFERENCE,
      type: NodeType.BLOCK,
      ck: "12345678-ffff-0000-0000-000000000000",
    },
    isSecret: false,
    isList: false,
    identityKey: "oEjRWeP//AAA=",
  },
];

describe("type encoding", () => {
  test.each(TEST_TYPE_IDENTITIES)("encode and decode type identity", (target) => {
    const identityKey = encodeTypeIdentity({ isSecret: false, isList: false, ...target });
    expect(identityKey).toBe(target.identityKey);

    const decoded = decodeTypeIdentity(identityKey);
    expect(decoded.primitiveType).toBe(target.primitiveType);
    expect(decoded.benchType).toBe(target.benchType);
    expect(getTkFromPtrMaybe(decoded.baseTypePtr)).toEqual(getTkFromPtrMaybe(target.baseTypePtr));
    expect(decoded.isList).toBe(target.isList);
    expect(decoded.isSecret).toBe(target.isSecret);
  });
});

describe("packing structs", () => {
  const OBJECT_TYPES_NAMES = OBJECT_TYPES.map((t) => ObjectType[t]);
  test.each(OBJECT_TYPES_NAMES)("roundtrip robust json %s", (typeName) => {
    const objectType = ObjectType[typeName as any] as unknown as ObjectType;
    const object = fabricate(objectType);
    const packedJson = _packStructValueScalar(object);
    const unpackedObject = _unpackStructValueScalar(packedJson);
    expect(unpackedObject).toEqual(object);
  });
});
