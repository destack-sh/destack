import { StatementModifier, SymbolType, TypeHint, TypeTag, type SimpleType } from "@/gql/graphql";
import { reverseRecord } from "@/utils/functools";

export const SYMBOL_TYPE_KEYWORD: Record<SymbolType, string> = {
  [SymbolType.Type]: "type",
  [SymbolType.Code]: "code",
  [SymbolType.Data]: "data",
  [SymbolType.Model]: "model",
  [SymbolType.Expectation]: "expect",
  [SymbolType.Task]: "task",
  [SymbolType.Capability]: "capability",
  [SymbolType.Requirement]: "require",
  [SymbolType.Build]: "build",
  [SymbolType.Block]: "block",
};
export const SUPPORTED_SYMBOL_TYPES = [
  SymbolType.Type,
  SymbolType.Code,
  SymbolType.Data,
  SymbolType.Model,
  SymbolType.Expectation,
  SymbolType.Task,
];
export const SYMBOL_TYPE_BY_KEYWORD: Record<string, SymbolType> = reverseRecord(SYMBOL_TYPE_KEYWORD);
export const MODIFIER_KEYWORD: Record<StatementModifier, string> = {
  [StatementModifier.Like]: "like",
  [StatementModifier.Unlike]: "unlike",
  [StatementModifier.Check]: "check",
  [StatementModifier.With]: "with",
  [StatementModifier.Var]: "vary",
  [StatementModifier.Local]: "local",
  [StatementModifier.Include]: "include",
  [StatementModifier.Magic]: "magic",
};
export const SUPPORTED_MODIFIERS = [StatementModifier.Like, StatementModifier.Unlike, StatementModifier.Check];
export const MODIFIER_BY_KEYWORD: Record<string, StatementModifier> = reverseRecord(MODIFIER_KEYWORD);
export const TYPETAG_KEYWORD: Record<TypeTag, string> = {
  [TypeTag.Boolean]: "boolean",
  [TypeTag.String]: "text",
  [TypeTag.Number]: "number",
  [TypeTag.File]: "file",
  [TypeTag.Embedding]: "embedding",
  [TypeTag.Json]: "json",
  [TypeTag.Literal]: "literal",
  [TypeTag.Struct]: "type",
  [TypeTag.Union]: "union",
};
export const TYPETAG_BY_KEYWORD: Record<string, TypeTag> = reverseRecord(TYPETAG_KEYWORD);
export const TYPEHINT_KEYWORD: Record<TypeHint, string> = {
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
export const SUPPORTED_TYPEHINTS: Record<TypeHint, TypeTag> = {
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
  } else {
    return null;
  }
  return builtin.slice(0, 1).toUpperCase() + builtin.slice(1); // always uppercase first letter
}

export const TYPENAME_SENTINEL = "__typename"; // :TypeSentinel
export const REMOTE_OBJECT_TYPENAME = "RemoteObject";
export const SECRET_TYPENAME = "Secret";

export function unkey(fields: SimpleType[], value: Record<string, any>): Record<string, any> {
  // TODO @Broken: unkey doesn't work with nested types
  const mapped: Record<string, any> = {};
  for (const field of fields) {
    if (field.key in value && field.name != null) {
      mapped[field.name] = value[field.key];
    }
  }
  return mapped;
}
