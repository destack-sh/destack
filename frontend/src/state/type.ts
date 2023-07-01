import { StatementType, TypeHint, TypeTag, type Field } from "@/gql/graphql";
import { TypeFlag } from "@/state/module";
import { reverseRecord } from "@/utils/functools";

export const STATEMENT_TYPE_KEYWORD: Partial<Record<StatementType, string>> = {
  [StatementType.Type]: "type",
  [StatementType.Task]: "task",
  [StatementType.Code]: "code",
  [StatementType.Value]: "value",
  [StatementType.Dataset]: "dataset",
  [StatementType.Model]: "model",
  [StatementType.Expectation]: "expect",
  [StatementType.Block]: "block",
};
export const SUPPORTED_STATEMENT_TYPES = [
  StatementType.Blank,
  StatementType.Text,
  StatementType.Type,
  StatementType.Code,
  StatementType.Dataset,
  StatementType.Value,
  StatementType.Model,
  StatementType.Expectation,
  StatementType.Task,
];
export const STATEMENT_TYPE_BY_KEYWORD: Partial<Record<string, StatementType>> = reverseRecord(STATEMENT_TYPE_KEYWORD);

export const TYPETAG_KEYWORD: Partial<Record<TypeTag, string>> = {
  [TypeTag.Boolean]: "boolean",
  [TypeTag.String]: "text",
  [TypeTag.Number]: "number",
  [TypeTag.File]: "file",
  [TypeTag.Vector]: "vector",
  [TypeTag.Json]: "json",
  [TypeTag.Literal]: "literal",
  [TypeTag.Struct]: "type",
  [TypeTag.Union]: "union",
};
export const TYPETAG_BY_KEYWORD: Partial<Record<string, TypeTag>> = reverseRecord(TYPETAG_KEYWORD);
export const TYPEHINT_KEYWORD: Partial<Record<TypeHint, string>> = {
  // string
  [TypeHint.Name]: "name",
  [TypeHint.Uuid]: "UUID",
  [TypeHint.Date]: "date",
  [TypeHint.Datetime]: "datetime",
  [TypeHint.Time]: "time",
  [TypeHint.Duration]: "duration",
  [TypeHint.Email]: "email",
  [TypeHint.Url]: "URL",
  [TypeHint.Markdown]: "markdown",
  [TypeHint.RichText]: "rich",
  [TypeHint.Html]: "HTML",
  [TypeHint.Code]: "code",
  [TypeHint.Key]: "key",
  [TypeHint.Phone]: "phone",
  [TypeHint.Secret]: "secret",
  // number
  [TypeHint.Integer]: "integer",
  [TypeHint.Float]: "float",
  [TypeHint.Slider]: "slider",
  [TypeHint.Rating]: "rating",
  // boolean
  [TypeHint.Toggle]: "toggle",
  [TypeHint.Checkbox]: "checkbox",
  [TypeHint.Thumbs]: "thumbs",
  // file
  [TypeHint.Image]: "image",
  [TypeHint.Audio]: "audio",
  [TypeHint.Video]: "video",
  // vector
  [TypeHint.Embedding]: "embedding",
};
export const SUPPORTED_TYPEHINTS: Partial<Record<TypeHint, TypeTag>> = {
  // string
  [TypeHint.Name]: TypeTag.String,
  [TypeHint.Uuid]: TypeTag.String,
  [TypeHint.Datetime]: TypeTag.String,
  [TypeHint.Url]: TypeTag.String,
  [TypeHint.Email]: TypeTag.String,
  [TypeHint.Html]: TypeTag.String,
  [TypeHint.Code]: TypeTag.String,
  [TypeHint.Key]: TypeTag.String,
  [TypeHint.Phone]: TypeTag.String,
  [TypeHint.Secret]: TypeTag.String,
  // number
  [TypeHint.Rating]: TypeTag.Number,
  [TypeHint.Integer]: TypeTag.Number,
  // boolean
  [TypeHint.Toggle]: TypeTag.Boolean,
  [TypeHint.Thumbs]: TypeTag.Boolean,
  // file
  // <only file for now>
  // vector
  [TypeHint.Embedding]: TypeTag.Vector,
};
export const TYPEHINT_BY_KEYWORD: Record<string, TypeHint> = reverseRecord(TYPEHINT_KEYWORD);

export function renderBuiltinType(tag: TypeTag, hint: TypeHint | null): string | null {
  let builtin = null;
  if (hint != null && hint in TYPEHINT_KEYWORD) {
    builtin = TYPEHINT_KEYWORD[hint];
  } else if (tag in TYPETAG_KEYWORD) {
    builtin = TYPETAG_KEYWORD[tag];
  }
  if (builtin == null) return null;
  return builtin.slice(0, 1).toUpperCase() + builtin.slice(1); // always uppercase first letter
}

export const DEFAULT_EMBEDDING_DIMENSION = 1536; // currently only support :FixedEmbeddingDimension
// sync with :TypeStorageFormat
export enum TypeStorageFormat {
  STRING = "str",
  DOUBLE = "f64",
  LONG = "s64",
  VECTOR = "vec",
  BINARY = "bin",
  BOOLEAN = "bool",
  DATE = "date",
  KEYWORD = "key",
  OBJECT = "obj",
  RELATION = "rel",
}

export const NATIVELY_SORTABLE_STORAGE_FORMATS = [
  TypeStorageFormat.DOUBLE,
  TypeStorageFormat.LONG,
  TypeStorageFormat.DATE,
  TypeStorageFormat.KEYWORD,
];

const STORAGE_FORMAT_BY_TYPE_TAG: Partial<{ [key in TypeTag]: TypeStorageFormat }> = {
  STRING: TypeStorageFormat.STRING,
  JSON: TypeStorageFormat.OBJECT,
  NUMBER: TypeStorageFormat.DOUBLE,
  BOOLEAN: TypeStorageFormat.BOOLEAN,
  VECTOR: TypeStorageFormat.VECTOR,
  FILE: TypeStorageFormat.OBJECT,
  STRUCT: TypeStorageFormat.OBJECT,
  ENUM: TypeStorageFormat.KEYWORD,
};

const STORAGE_FORMAT_BY_TYPE_HINT: Partial<{ [key in TypeHint]: TypeStorageFormat }> = {
  UUID: TypeStorageFormat.KEYWORD,
  DATE: TypeStorageFormat.DATE,
  DATETIME: TypeStorageFormat.DATE,
  TIME: TypeStorageFormat.LONG,
  DURATION: TypeStorageFormat.DOUBLE,
  KEY: TypeStorageFormat.KEYWORD,
  INTEGER: TypeStorageFormat.LONG,
  FLOAT: TypeStorageFormat.DOUBLE,
};

export function getStorageFormat(
  tag: TypeTag,
  hint: TypeHint | undefined | null,
  flags: TypeFlag
): TypeStorageFormat | undefined {
  if (flags & TypeFlag.IsSecret) {
    return TypeStorageFormat.OBJECT;
  }
  if (hint != null && hint in STORAGE_FORMAT_BY_TYPE_HINT) {
    return STORAGE_FORMAT_BY_TYPE_HINT[hint];
  }
  return STORAGE_FORMAT_BY_TYPE_TAG[tag];
}

export type TypeIndexInfo = {
  tag: TypeTag;
  hint: TypeHint | null;
  flags: TypeFlag;
  format: TypeStorageFormat | null;
  sortable: boolean;
  subfields: TypeIndexInfo[] | null;
};

export const TYPENAME_SENTINEL = "__typename"; // :TypeSentinel
export const REMOTE_OBJECT_TYPENAME = "RemoteObject";
export const SECRET_TYPENAME = "Secret";

export function unkey(fields: Field[], value: Record<string, any>): Record<string, any> {
  // TODO @Broken: unkey doesn't work with nested types, should probably be in module
  const mapped: Record<string, any> = {};
  for (const field of fields) {
    if (field.key in value && field.name != null) {
      mapped[field.name] = value[field.key];
    }
  }
  return mapped;
}

export enum SubfieldType {
  key = "key",
  starts_with = "starts_with",
  token_count = "token_count",
  char_count = "char_count",
}

export function getMainSubfield(field: Pick<Field, "hint" | "tag">): SubfieldType | null {
  if (field.hint == TypeHint.Name) {
    return SubfieldType.key;
  } else if (field.tag == TypeTag.String) {
    return SubfieldType.char_count;
  }
  return null;
}

export function canSort(
  field: Pick<Field, "tag" | "hint" | "flags">,
  options?: { excludeSubfields?: boolean }
): boolean {
  const storageFormat = getStorageFormat(field.tag, field.hint, field.flags);
  if (storageFormat == null) return false;
  if (NATIVELY_SORTABLE_STORAGE_FORMATS.includes(storageFormat)) return true;
  if (options?.excludeSubfields) return false;
  const subfield = getMainSubfield(field);
  return subfield != null;
}
