import { TypeHint, TypeTag, type Field, StatementType } from "@/gql/graphql";
import { TypeFlag } from "@/state/module";
import { getStatementIconOutline } from "@/state/statement";
import { reverseRecord } from "@/utils/functools";

export const TYPETAG_KEYWORD: Partial<Record<TypeTag, string>> = {
  [TypeTag.Boolean]: "boolean",
  [TypeTag.String]: "text",
  [TypeTag.Number]: "number",
  [TypeTag.Blob]: "blob",
  [TypeTag.Vector]: "vector",
  [TypeTag.Json]: "json",
  [TypeTag.Literal]: "literal",
  [TypeTag.Struct]: "type",
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
  [TypeHint.RichText]: "rich text",
  [TypeHint.Html]: "HTML",
  [TypeHint.Code]: "code",
  [TypeHint.Key]: "key",
  [TypeHint.Phone]: "phone",
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
  // node
  [TypeHint.File]: "file",
  [TypeHint.Statement]: "statement",
  [TypeHint.Run]: "run",
  [TypeHint.Secret]: "secret",
  // [TypeHint.Blob]: "blob",
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
  [TypeHint.RichText]: TypeTag.String,
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
  // node
  [TypeHint.File]: TypeTag.Node,
  [TypeHint.Statement]: TypeTag.Node,
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

export const DEFAULT_EMBEDDING_DIMENSION = 768; // currently only support :FixedEmbeddingDimension
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

export const SORTABLE_STORAGE_FORMATS = [
  TypeStorageFormat.STRING,
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
  BLOB: TypeStorageFormat.OBJECT,
  STRUCT: TypeStorageFormat.OBJECT,
  ENUM: TypeStorageFormat.KEYWORD,
  LITERAL: TypeStorageFormat.KEYWORD,
  NODE: TypeStorageFormat.RELATION,
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
  if (flags & TypeFlag.IS_SECRET) {
    return TypeStorageFormat.OBJECT;
  }
  if (hint != null && hint in STORAGE_FORMAT_BY_TYPE_HINT) {
    return STORAGE_FORMAT_BY_TYPE_HINT[hint];
  }
  return STORAGE_FORMAT_BY_TYPE_TAG[tag];
}

export type TypeStorageInfo = {
  tag: TypeTag;
  hint: TypeHint | null;
  flags: TypeFlag;
  format: TypeStorageFormat | null;
  sortable: boolean;
};

export const BLOB_TYPENAME = "Blob";
export const SECRET_TYPENAME = "Secret";

export function canSort(field: Pick<Field, "tag" | "hint" | "flags">): boolean {
  const storageFormat = getStorageFormat(field.tag, field.hint, field.flags);
  if (storageFormat == null) return false;
  if (SORTABLE_STORAGE_FORMATS.includes(storageFormat)) return true;
  return false;
}
import {
  AdjustmentsHorizontalIcon,
  ArrowsRightLeftIcon,
  AtSymbolIcon,
  Bars3BottomLeftIcon,
  CalendarDaysIcon,
  CheckIcon,
  ClockIcon,
  CodeBracketIcon,
  DocumentIcon,
  DocumentTextIcon,
  FingerPrintIcon,
  HandThumbUpIcon,
  HashtagIcon,
  IdentificationIcon,
  KeyIcon,
  LinkIcon,
  LockClosedIcon,
  PhoneIcon,
  PhotoIcon,
  SparklesIcon,
  SpeakerWaveIcon,
  StarIcon,
  VideoCameraIcon,
} from "@heroicons/vue/24/outline";

export const ICONS_BY_TAG_OUTLINE: Partial<Record<TypeTag, any>> = {
  [TypeTag.String]: Bars3BottomLeftIcon,
  [TypeTag.Number]: HashtagIcon,
  [TypeTag.Boolean]: CheckIcon,
  [TypeTag.Vector]: SparklesIcon,
  [TypeTag.Blob]: DocumentIcon,
  [TypeTag.Struct]: getStatementIconOutline(StatementType.Class),
  [TypeTag.Enum]: getStatementIconOutline(StatementType.Choice),
  [TypeTag.Node]: CodeBracketIcon,
};
export const ICONS_BY_HINT_OUTLINE: Partial<Record<TypeHint, any>> = {
  // string
  [TypeHint.Name]: IdentificationIcon,
  [TypeHint.Uuid]: FingerPrintIcon,
  [TypeHint.Date]: CalendarDaysIcon,
  [TypeHint.Datetime]: CalendarDaysIcon,
  [TypeHint.Time]: ClockIcon,
  [TypeHint.Duration]: ClockIcon,
  [TypeHint.Url]: LinkIcon,
  [TypeHint.Email]: AtSymbolIcon,
  [TypeHint.Markdown]: CodeBracketIcon,
  [TypeHint.Html]: CodeBracketIcon,
  [TypeHint.Code]: CodeBracketIcon,
  [TypeHint.Key]: KeyIcon,
  [TypeHint.Secret]: LockClosedIcon,
  [TypeHint.RichText]: DocumentTextIcon,
  // number
  [TypeHint.Integer]: HashtagIcon, // should have a different icon from float
  [TypeHint.Float]: HashtagIcon,
  [TypeHint.Slider]: AdjustmentsHorizontalIcon,
  [TypeHint.Phone]: PhoneIcon,
  [TypeHint.Rating]: StarIcon,
  // boolean
  [TypeHint.Toggle]: ArrowsRightLeftIcon,
  [TypeHint.Checkbox]: CheckIcon,
  [TypeHint.Thumbs]: HandThumbUpIcon,
  // file
  [TypeHint.Audio]: SpeakerWaveIcon,
  [TypeHint.Video]: VideoCameraIcon,
  [TypeHint.Image]: PhotoIcon,
};
