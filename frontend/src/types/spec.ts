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
};

export type EnumType = _Type & {
  _type: "EnumType";
  values: any[];
};

export type FieldType = ValueType | EnumType;
const FIELD_TYPES = ["ValueType", "EnumType"];

export function isFieldType(obj: any) {
  return FIELD_TYPES.includes(obj._type);
}

export type FieldValuePrimitive = string | number | object;
export type FieldValue = any;

export type FieldSpec = _Spec & {
  _type: "FieldSpec";
  type: FieldType | FieldSpec | Array<FieldSpec> | Record<string, FieldSpec>;
};

export function isFieldSpec(obj: any) {
  return obj._type == "FieldSpec";
}

export function makeFieldSpec(
  name: string,
  type: FieldType | Array<FieldSpec> | Record<string, FieldSpec>
): FieldSpec {
  return { _type: "FieldSpec", name, type };
}

export type RecordSpec = FieldSpec;

export type ArtifactSpec = _Spec & {
  type: string;
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
  type: Record<string, ArtifactSpec | FieldSpec>;
};

const CONFIG_SPEC_TYPES = ["DatasetSpec", "ModelSpec", "FieldSpec"];
export function isConfigSpecType(obj: any) {
  return CONFIG_SPEC_TYPES.includes(obj._type);
}

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
