type _Type = {
  _type: string;
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

export type FieldSpec = _Spec & {
  _type: "FieldSpec";
  type: FieldType | Array<FieldSpec> | Record<string, FieldSpec>;
};

export function isFieldSpec(obj: any) {
  return obj._type == "FieldSpec";
}

export type RecordSpec = FieldSpec;

export type ArtifactSpec = _Spec & {
  type: string;
};

export type DatasetSpec = ArtifactSpec & {
  _type: "DatasetSpec";
  type: "dataset";
  record_spec: RecordSpec;
  config_spec: ConfigSpec;
};

export type ModelSpec = ArtifactSpec & {
  _type: "ModelSpec";
  type: "model";
  input_spec: RecordSpec;
  output_spec: RecordSpec;
  config_spec: ConfigSpec;
};

export type ConfigSpec = _Spec & {
  _type: "ConfigSpec";
  type: Record<string, ArtifactSpec | FieldSpec>;
};

const CONFIG_SPEC_TYPES = ["DatasetSpec", "ModelSpec", "FieldSpec"];
export function isConfigSpecType(obj: any) {
  return CONFIG_SPEC_TYPES.includes(obj._type);
}

export type ModelHandlerSpec = _Spec & {
  _type: "ModelHandlerSpec";
  base_spec: ModelSpec;
  config_spec: ConfigSpec;
};
