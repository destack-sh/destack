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
};

export type FieldType = {
  _type: string;
};

export type FieldSpec = _Spec & {
  type: FieldType | Array<FieldSpec> | Record<string, FieldSpec>;
};

export type RecordSpec = FieldSpec;

export type ArtifactSpec = _Spec & {
  type: string;
};

export type DatasetSpec = ArtifactSpec & {
  type: "dataset";
  record_spec: RecordSpec;
  config_spec: ConfigSpec;
};

export type ModelSpec = ArtifactSpec & {
  type: "model";
  input_spec: RecordSpec;
  output_spec: RecordSpec;
  config_spec: ConfigSpec;
};

export type ConfigSpec = _Spec & {
  type: Record<string, ArtifactSpec | FieldSpec>;
};

export type ModelHandlerSpec = _Spec & {
  base_spec: ModelSpec;
  config_spec: ConfigSpec;
};
