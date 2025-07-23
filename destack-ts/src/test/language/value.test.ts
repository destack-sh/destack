import { expect, test } from "bun:test";
import { PrimitiveType, ScalarType, Type, TypeCardinality, toType } from "@destack/language";

test("to type", () => {
  // parse values and types into Types

  // scalar values
  expect(toType(1.0)).toEqual(
    new Type({
      cardinality: TypeCardinality.SCALAR,
      scalarType: ScalarType.PRIMITIVE,
      primitiveType: PrimitiveType.FLOAT64,
    }),
  );
  expect(toType(17)).toEqual(
    new Type({
      cardinality: TypeCardinality.SCALAR,
      scalarType: ScalarType.PRIMITIVE,
      primitiveType: PrimitiveType.FLOAT64,
    }),
  );
  expect(toType("17")).toEqual(
    new Type({
      cardinality: TypeCardinality.SCALAR,
      scalarType: ScalarType.PRIMITIVE,
      primitiveType: PrimitiveType.STRING,
    }),
  );
  expect(toType(true)).toEqual(
    new Type({
      cardinality: TypeCardinality.SCALAR,
      scalarType: ScalarType.PRIMITIVE,
      primitiveType: PrimitiveType.BOOLEAN,
    }),
  );

  // collection values
  expect(toType([1, 2, 3])).toEqual(
    new Type({
      cardinality: TypeCardinality.LIST,
      scalarType: ScalarType.PRIMITIVE,
      primitiveType: PrimitiveType.FLOAT64,
    }),
  );
  expect(toType({ a: 1, b: 2, c: 3 })).toEqual(
    new Type({
      cardinality: TypeCardinality.MAP,
      scalarType: ScalarType.PRIMITIVE,
      primitiveType: PrimitiveType.FLOAT64,
      keyType: new Type({
        cardinality: TypeCardinality.SCALAR,
        scalarType: ScalarType.PRIMITIVE,
        primitiveType: PrimitiveType.STRING,
      }),
    }),
  );
});
