import { EnumType } from "@destack/language/core/builtin/builtin";
import { registerEnumClass } from "@destack/language/registry";

/* ==== DESTACK_GENERATED_START:ENUM:31 ==== */
/**
 * RuntimeLanguage
 */
export enum RuntimeLanguage {
  PYTHON = 1,
  JAVASCRIPT = 2,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.RUNTIME_LANGUAGE, RuntimeLanguage);
/* ==== DESTACK_GENERATED_END:ENUM:31 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:30 ==== */
/**
 * PlatformType
 */
export enum PlatformType {
  SYSTEM = 1,
  RUNTIME = 2,
  WEB = 10,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.PLATFORM_TYPE, PlatformType);
/* ==== DESTACK_GENERATED_END:ENUM:30 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:40 ==== */
/**
 * OperatingSystem
 */
export enum OperatingSystem {
  LINUX = 1,
  WINDOWS = 2,
  MACOS = 3,
  ANDROID = 50,
  IOS = 51,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.OPERATING_SYSTEM, OperatingSystem);
/* ==== DESTACK_GENERATED_END:ENUM:40 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:20 ==== */
/**
 * GraphKey
 */
export enum GraphKey {
  ENTITY_PRIMARY = 1110,
  EVENT_PRIMARY = 2110,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.GRAPH_KEY, GraphKey);
/* ==== DESTACK_GENERATED_END:ENUM:20 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:1100000 ==== */
/**
 * EnvironmentType
 */
export enum EnvironmentType {
  SYSTEM = 1,
  DEVELOPMENT = 3,
  TEST = 5,
  STAGING = 7,
  PRODUCTION = 10,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.ENVIRONMENT_TYPE, EnvironmentType);
/* ==== DESTACK_GENERATED_END:ENUM:1100000 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:1000000 ==== */
/**
 * Cloud
 */
export enum Cloud {
  PRIVATE = 1,
  AWS = 10,
  AZURE = 11,
  GCP = 12,
  HETZNER = 20,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.CLOUD, Cloud);
/* ==== DESTACK_GENERATED_END:ENUM:1000000 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:1000003 ==== */
/**
 * RegionContinent
 */
export enum RegionContinent {
  EUROPE = 1000,
  NORTH_AMERICA = 2000,
  SOUTH_AMERICA = 3000,
  MIDDLE_EAST = 4000,
  AFRICA = 5000,
  ASIA = 6000,
  AUSTRALIA = 7000,
  PRIVATE = 9000,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.REGION_CONTINENT, RegionContinent);
/* ==== DESTACK_GENERATED_END:ENUM:1000003 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:1000002 ==== */
/**
 * RegionArea
 */
export enum RegionArea {
  EUROPE_CENTRAL = 1000,
  NORTH_AMERICA_EAST = 2000,
  NORTH_AMERICA_WEST = 2200,
  SOUTH_AMERICA_EAST = 3000,
  MIDDLE_EAST_CENTRAL = 4000,
  MIDDLE_EAST_WEST = 4200,
  AFRICA_SOUTH = 5000,
  ASIA_WEST = 6000,
  ASIA_SOUTH = 6200,
  ASIA_EAST = 6400,
  AUSTRALIA_SOUTH = 7000,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.REGION_AREA, RegionArea);
/* ==== DESTACK_GENERATED_END:ENUM:1000002 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:1000001 ==== */
/**
 * Region
 */
export enum Region {
  ZURICH = 1000,
  FRANKFURT = 1010,
  VIRGINIA = 2000,
  OHIO = 2010,
  OREGON = 2200,
  SAO_PAULO = 3000,
  CAPE_TOWN = 5000,
  MUMBAI = 6000,
  SINGAPORE = 6200,
  TOKYO = 6400,
  SYDNEY = 7000,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.REGION, Region);
/* ==== DESTACK_GENERATED_END:ENUM:1000001 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:300200 ==== */
/**
 * RoleType
 */
export enum RoleType {
  SYSTEM = 1,
  OWNER = 2,
  ADMIN = 3,
  DEVELOPER = 5,
  USER = 7,
  SPECTATOR = 10,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.ROLE_TYPE, RoleType);
/* ==== DESTACK_GENERATED_END:ENUM:300200 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:1000004 ==== */
/**
 * Tenancy
 */
export enum Tenancy {
  DEDICATED = 1,
  SHARED = 2,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.TENANCY, Tenancy);
/* ==== DESTACK_GENERATED_END:ENUM:1000004 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:106 ==== */
/**
 * PropertyType
 */
export enum PropertyType {
  MEMBER = 1,
  CONSTANT = 2,
  INPUT = 10,
  OUTPUT = 11,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.PROPERTY_TYPE, PropertyType);
/* ==== DESTACK_GENERATED_END:ENUM:106 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:107 ==== */
/**
 * EdgeType
 */
export enum EdgeType {
  PARENT = 1,
  REGULAR = 5,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.EDGE_TYPE, EdgeType);
/* ==== DESTACK_GENERATED_END:ENUM:107 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:109 ==== */
/**
 * CascadeAction
 */
export enum CascadeAction {
  RESTRICT = 1,
  CASCADE = 2,
  SET_NULL = 3,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.CASCADE_ACTION, CascadeAction);
/* ==== DESTACK_GENERATED_END:ENUM:109 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:108 ==== */
/**
 * EdgeDirection
 */
export enum EdgeDirection {
  PARENT = 1,
  CHILD = 2,
  DEFINITION = 10,
  INSTANCE = 11,
  SIDE = 20,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.EDGE_DIRECTION, EdgeDirection);
/* ==== DESTACK_GENERATED_END:ENUM:108 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:100 ==== */
/**
 * PrimitiveType
 */
export enum PrimitiveType {
  BOOLEAN = 2,
  SINT8 = 10,
  SINT16 = 11,
  SINT32 = 12,
  SINT64 = 13,
  SINT128 = 14,
  UINT8 = 15,
  UINT16 = 16,
  UINT32 = 17,
  UINT64 = 18,
  UINT128 = 19,
  FLOAT16 = 21,
  FLOAT32 = 22,
  FLOAT64 = 23,
  DATETIME = 30,
  DATE = 31,
  TIME = 32,
  DURATION = 33,
  STRING = 40,
  UUID = 41,
  BYTES = 42,
  JSON = 45,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.PRIMITIVE_TYPE, PrimitiveType);
/* ==== DESTACK_GENERATED_END:ENUM:100 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:101 ==== */
/**
 * TypeCardinality
 */
export enum TypeCardinality {
  SCALAR = 1,
  LIST = 2,
  MAP = 5,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.TYPE_CARDINALITY, TypeCardinality);
/* ==== DESTACK_GENERATED_END:ENUM:101 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:102 ==== */
/**
 * ScalarType
 */
export enum ScalarType {
  PRIMITIVE = 1,
  ENUM = 2,
  NODE_REFERENCE = 3,
  NODE_VALUE = 4,
  STRUCT = 5,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.SCALAR_TYPE, ScalarType);
/* ==== DESTACK_GENERATED_END:ENUM:102 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:103 ==== */
/**
 * ValueFactory
 */
export enum ValueFactory {
  UUID4 = 1,
  UUID7 = 2,
  NOW = 10,
  EPOCH = 11,
  ACTOR = 12,
  CLIENT = 13,
  CLIENT_NONCE = 14,
  REGION = 20,
  SELF = 30,
  SPACE = 31,
  BRANCH = 32,
  SNAPSHOT = 33,
  NAME = 40,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.VALUE_FACTORY, ValueFactory);
/* ==== DESTACK_GENERATED_END:ENUM:103 ==== */

export const PRIMITIVE_TYPE_BY_JS_TYPE_NAME: Map<string, PrimitiveType> = new Map([
  ["Boolean", PrimitiveType.BOOLEAN],
  ["Number", PrimitiveType.FLOAT64], // default for Number
  ["String", PrimitiveType.STRING],
  ["Uint8Array", PrimitiveType.BYTES],
  ["ZonedDateTime", PrimitiveType.DATETIME],
  ["PlainDate", PrimitiveType.DATE],
  ["PlainTime", PrimitiveType.TIME],
  ["Duration", PrimitiveType.DURATION],
] as any);

/* ==== DESTACK_GENERATED_START:ENUM:110 ==== */
/**
 * Encoding
 */
export enum Encoding {
  JSON = 1,
  CSON = 2,
  KOMPAKT = 3,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.ENCODING, Encoding);
/* ==== DESTACK_GENERATED_END:ENUM:110 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:121300 ==== */
/**
 * ClientType
 */
export enum ClientType {
  WEB = 1,
  BROWSER_PLUGIN = 2,
  DESKTOP = 3,
  MOBILE = 4,
  MACHINE = 10,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.CLIENT_TYPE, ClientType);
/* ==== DESTACK_GENERATED_END:ENUM:121300 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:21 ==== */
/**
 * GraphDomain
 */
export enum GraphDomain {
  ENTITY = 1,
  EVENT = 2,

  /* ==== DESTACK_CUSTOM_START ==== */
  /* ... */
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.GRAPH_DOMAIN, GraphDomain);
/* ==== DESTACK_GENERATED_END:ENUM:21 ==== */
