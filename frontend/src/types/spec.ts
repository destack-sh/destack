type _Type = {
  _type: string;
  optional?: boolean;
};

type _Spec = {
  name: string;
  description?: string;
};

export type ValueType = _Type & {
  _type: "ValueType";
  dtype: string;
  default?: any;
  optional?: boolean;
};

export type EnumType = _Type & {
  _type: "EnumType";
  values: any[];
  optional?: boolean;
};

export type FieldTypePrimitive = ValueType | EnumType;
const FIELD_TYPES = ["ValueType", "EnumType"];

export function isFieldType(obj: any) {
  return FIELD_TYPES.includes(obj._type);
}

export type FieldValuePrimitive = string | number | object;
export type FieldValue = any;

export type FieldType =
  | FieldTypePrimitive
  | FieldSpec
  | Array<FieldSpec>
  | Record<string, FieldSpec>;
export type FieldSpec = _Spec & {
  _type: "FieldSpec";
  type: FieldType | FieldSpec | Array<FieldSpec> | Record<string, FieldSpec>;
};

export type RecordSpec = FieldSpec;
export type RecordType = FieldType;

export function isFieldSpec(obj: any) {
  return obj._type == "FieldSpec";
}

export function makeFieldSpec(
  name: string,
  type: FieldType | Array<FieldSpec> | Record<string, FieldSpec>
): FieldSpec {
  return { _type: "FieldSpec", name, type };
}

// Complex types

export type ArtifactType = _Type;
export type ModelType = ArtifactSpec & {
  _type: "ModelType";
  input_spec: RecordSpec | RecordType;
  output_spec: RecordSpec | RecordType;
};

export type DatasetType = ArtifactSpec & {
  _type: "DatasetType";
  record_spec: RecordSpec | RecordType;
};

const ARTIFACT_TYPES = ["ModelType", "DatasetType"];
export function isArtifactType(obj: any) {
  return ARTIFACT_TYPES.includes(obj._type);
}

export type ArtifactSpec = _Spec & {
  _type: "ArtifactSpec";
};

export type DatasetSpec = ArtifactSpec & {
  _type: "DatasetSpec";
  record_spec: RecordSpec;
};

export type ModelSpec = ArtifactSpec & {
  _type: "ModelSpec";
  input_spec: RecordSpec;
  output_spec: RecordSpec;
};

export type FunctionSpec = _Spec & {
  _type: "FunctionSpec";
  input_spec: Record<string, RecordSpec>;
  output_spec: Record<string, RecordSpec>;
};

export type ConfigSpec = _Spec & {
  _type: "ConfigSpec";
  type: Record<string, FieldSpec>;
};

export type DatasetHandlerSpec = _Spec & {
  _type: "DatasetHandlerSpec";
  base_spec: DatasetSpec;
  config_spec: RecordSpec;
};

export type ModelHandlerSpec = _Spec & {
  _type: "ModelHandlerSpec";
  base_spec: ModelSpec;
  config_spec: RecordSpec;
};

export type FunctionType = "RecordTransform" | "Metric" | "Test";

export type FunctionHandlerSpec = {
  name: string;
  description: string;
  type: FunctionType;
  config_spec: ConfigSpec;
  base_spec: FunctionSpec;
};

// Helper methods

export function unravelConfigSpec(spec: ConfigSpec): FieldSpec[] {
  return Object.values(spec.type);
}

export function unravelFieldSpec(spec: FieldSpec | FieldSpec[]): FieldSpec[] {
  if (Array.isArray(spec)) {
    return spec.flatMap(unravelFieldSpec);
  } else if (isFieldType(spec.type)) {
    return [spec];
  } else if (isFieldSpec(spec.type)) {
    // if the spec type is just a blank spec it's just a level of annotation
    return unravelFieldSpec(spec.type as FieldSpec);
  } else if (Array.isArray(spec.type)) {
    return spec.type;
  } else {
    return Object.values(spec.type);
  }
}

export function reduceToFieldSpec(spec: ConfigSpec): FieldSpec[] {
  return unravelConfigSpec(spec)
    .filter((spec) => isFieldType(spec.type))
    .flatMap((spec) => unravelFieldSpec(spec as FieldSpec));
}

const ARTIFACT_SPEC_TYPES = ["DatasetSpec", "ModelSpec"];
const CONFIG_SPEC_TYPES = [...ARTIFACT_SPEC_TYPES, "FieldSpec"];

export function isArtifactSpecType(obj: any): boolean {
  return ARTIFACT_SPEC_TYPES.includes(obj._type);
}

export function isConfigSpecType(obj: any) {
  return CONFIG_SPEC_TYPES.includes(obj._type);
}
