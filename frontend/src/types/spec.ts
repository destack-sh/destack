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

export type FieldType = FieldTypePrimitive | FieldSpec | Array<FieldSpec> | Record<string, FieldSpec>;
export type FieldSpec = _Spec & {
  _type: "FieldSpec";
  type: FieldType | FieldSpec | Array<FieldSpec> | Record<string, FieldSpec>;
};

export type RecordSpec = FieldSpec;
export type RecordType = FieldType;

export function makeFieldSpec(name: string, type: FieldType | Array<FieldSpec> | Record<string, FieldSpec>): FieldSpec {
  return { _type: "FieldSpec", name, type };
}

// Complex types

export type ArtifactType = _Type;
export type ModelType = ArtifactSpec & {
  _type: "ModelType";
  input_spec: RecordSpec | RecordType;
  output_spec: RecordSpec | RecordType;
  optiona?: boolean;
};

export type DatasetType = ArtifactSpec & {
  _type: "DatasetType";
  record_spec: RecordSpec | RecordType;
  optiona?: boolean;
};

export type FunctionType = _Type & {
  input_spec: Record<string, RecordSpec>;
  output_spec: Record<string, RecordSpec>;
  optiona?: boolean;
};

const ARTIFACT_TYPES = ["ModelType", "DatasetType"];
export function isArtifactType(obj: any) {
  return ARTIFACT_TYPES.includes(obj._type);
}

export type ArtifactSpec = _Spec & {
  _type: "ArtifactSpec";
  type: ArtifactType;
};

export type FunctionSpec = _Spec & {
  _type: "FunctionSpec";
  type: FunctionType;
};

export type AnySpec = FieldSpec | ArtifactSpec;
export type ConfigType = Record<string, AnySpec>;

export type ConfigSpec = _Spec & {
  _type: "ConfigSpec";
  type: ConfigType;
};

export function isFieldSpec(obj: any) {
  return obj._type == "FieldSpec";
}

export function isConfigSpec(obj: any) {
  return obj._type == "ConfigSpec";
}

export function isArtifactSpec(obj: any) {
  return obj._type == "DatasetSpec" || obj._type == "ModelSpec";
}

export function isAnySpec(obj: any) {
  return ["FieldSpec", "DatasetSpec", "ModelSpec"].includes(obj._type);
}

export type DatasetHandlerSpec = _Spec & {
  _type: "DatasetHandlerSpec";
  id: string;
  tags: string[];
  base_spec: DatasetType;
  config_spec: Record<string, FieldSpec>;
};

export type ModelHandlerSpec = _Spec & {
  _type: "ModelHandlerSpec";
  id: string;
  tags: string[];
  base_spec: ModelType;
  config_spec: Record<string, FieldSpec>;
};

export type FunctionHandlerType = "RecordTransform" | "Metric" | "Test";

export type FunctionHandlerSpec = _Spec & {
  id: string;
  tags: string[];
  type: FunctionHandlerType;
  base_spec: FunctionType;
  config_spec: ConfigType;
};

// Helper methods

export function unravelSpec(spec: AnySpec | AnySpec[]): AnySpec[] {
  if (Array.isArray(spec)) {
    return spec.flatMap(unravelSpec);
  } else if (isFieldSpec(spec)) {
    if (isFieldType(spec.type)) {
      return [spec];
    } else if (isFieldSpec(spec) && isFieldSpec(spec.type)) {
      // if the spec type is just a blank spec it's just a level of annotation
      return unravelSpec(spec.type as FieldSpec);
    } else if (Array.isArray(spec.type)) {
      return spec.type;
    } else {
      return Object.values(spec.type);
    }
  } else {
    // don't unravel non-field specs (nothing to unravel)
    return [spec];
  }
}

export function reduceToFieldSpec(spec: AnySpec[]): FieldSpec[] {
  return unravelSpec(Object.values(spec))
    .filter((spec) => isFieldSpec(spec))
    .flatMap((spec) => unravelSpec(spec as FieldSpec) as FieldSpec[]);
}
