import { ExpectationModifier, StatementType, TypeHint, TypeTag, type Field } from "@/gql/graphql";
import { reverseRecord } from "@/utils/functools";

export const STATEMENT_TYPE_KEYWORD: Partial<Record<StatementType, string>> = {
  [StatementType.Type]: "type",
  [StatementType.Task]: "task",
  [StatementType.Code]: "code",
  [StatementType.Value]: "value",
  [StatementType.Dataset]: "database",
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
export const MODIFIER_KEYWORD: Record<ExpectationModifier, string> = {
  [ExpectationModifier.Like]: "like",
  [ExpectationModifier.Unlike]: "unlike",
  [ExpectationModifier.Check]: "check",
};
export const SUPPORTED_MODIFIERS = [ExpectationModifier.Like, ExpectationModifier.Unlike, ExpectationModifier.Check];
export const MODIFIER_BY_KEYWORD: Record<string, ExpectationModifier> = reverseRecord(MODIFIER_KEYWORD);
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
  // number
  [TypeHint.Integer]: "integer",
  [TypeHint.Float]: "float",
  [TypeHint.Slider]: "slider",
  [TypeHint.Phone]: "phone",
  [TypeHint.Rating]: "rating",
  // boolean
  [TypeHint.Toggle]: "toggle",
  [TypeHint.Checkbox]: "checkbox",
  [TypeHint.Thumbs]: "thumbs",
  // file
  [TypeHint.Image]: "image",
  [TypeHint.Audio]: "audio",
  [TypeHint.Video]: "video",
};
export const SUPPORTED_TYPEHINTS: Partial<Record<TypeHint, TypeTag>> = {
  // string
  [TypeHint.Name]: TypeTag.String,
  [TypeHint.Uuid]: TypeTag.String,
  [TypeHint.Datetime]: TypeTag.String,
  [TypeHint.Url]: TypeTag.String,
  [TypeHint.Email]: TypeTag.String,
  [TypeHint.Html]: TypeTag.String,
  [TypeHint.Key]: TypeTag.String,
  [TypeHint.Code]: TypeTag.String,
  // number
  [TypeHint.Phone]: TypeTag.Number,
  [TypeHint.Rating]: TypeTag.Number,
  [TypeHint.Integer]: TypeTag.Number,
  // boolean
  [TypeHint.Toggle]: TypeTag.Boolean,
  [TypeHint.Thumbs]: TypeTag.Boolean,
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

export const TYPENAME_SENTINEL = "__typename"; // :TypeSentinel
export const REMOTE_OBJECT_TYPENAME = "RemoteObject";
export const SECRET_TYPENAME = "Secret";

export function unkey(fields: Field[], value: Record<string, any>): Record<string, any> {
  // TODO @Broken: unkey doesn't work with nested types
  const mapped: Record<string, any> = {};
  for (const field of fields) {
    if (field.key in value && field.name != null) {
      mapped[field.name] = value[field.key];
    }
  }
  return mapped;
}
