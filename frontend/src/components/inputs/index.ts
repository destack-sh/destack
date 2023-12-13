import { TypeHint, TypeTag } from "@/gql/graphql";
import { isValidBlobRecord } from "@/state/blob";
import { TypeFlag, type Field } from "@/state/module";
import { isValidSecretRecord } from "@/state/secret";

export type ValueInterface = {
  id: string;
  tags?: TypeTag[];
  hints?: TypeHint[];
  debounceMs?: number;
  supportsList?: boolean;
  isSecret?: boolean;
  // read/write mapping
  read?(type: Field, value: any): any;
  write?(type: Field, value: any): any;
  map?(type: Field, value: any): any;
  // display and sizing (see table in data cell)
  minWidth?: number;
  grow?: number;
  inline?: boolean;
};

export const interfaces: Record<string, ValueInterface> = {};

export function registerInterface(id: string, value: Omit<ValueInterface, "id">) {
  if (interfaces[id] != null) {
    throw new Error(`interface ${id} already registered`);
  }
  // if map is set, set it as read and write
  if (value.map != null) {
    value.read = value.map;
    value.write = value.map;
  }
  interfaces[id] = { ...value, id };
}

// We no longer auto-coerce to/from arrays as needed because of real databases. :NoArrayCoercion

function toArray(type: Field, value: any): any[] {
  const isArray = Array.isArray(value);
  const shouldArray = type.flags & TypeFlag.IS_ARRAY || type.flags & TypeFlag.IS_ARRAYABLE;
  if (shouldArray) {
    return isArray ? value : [];
  } else {
    return isArray ? [] : [value];
  }
}

function coerceToScalarOrArray(type: Field, value: any): any[] | any {
  /* Coerce OUTPUT values as needed for the storage format */
  const isArray = Array.isArray(value);
  const shouldArray = type.flags & TypeFlag.IS_ARRAY || type.flags & TypeFlag.IS_ARRAYABLE;
  if (shouldArray) {
    return isArray ? value : [value];
  } else {
    return isArray ? value[0] : value;
  }
}

function coerceToBoolean(type: Field, value: any) {
  return typeof value == "boolean" ? value : false;
}

function coerceToString(type: Field, value: any) {
  if (typeof value == "number") {
    value = value.toString();
  }
  return typeof value == "string" ? value : "";
}

function coerceToDatetime(type: Field, value: any) {
  if (typeof value != "string") return null;
  // expected format is YYYY-MM-DDTHH:MM:SS
  // strip timezone
  value = value.replace(/(\.\d+)?(Z|[+-]\d{2}:\d{2})$/, "");
  // strip milliseconds
  value = value.replace(/\.\d+$/, "");
  return value;
}

function coerceToNumber(type: Field, value: any) {
  if (typeof value == "string") {
    value = Number.parseFloat(value.trim());
  }
  return typeof value == "number" ? value : null;
}

// string
registerInterface("string", {
  tags: [TypeTag.String],
  map: coerceToString,
  debounceMs: 1000,
  minWidth: 200,
  grow: 1.0,
});
registerInterface("string.code", {
  hints: [TypeHint.Code, TypeHint.Html],
  map: coerceToString,
  debounceMs: 2000,
  minWidth: 200,
  grow: 1.0,
});
registerInterface("string.rich", {
  hints: [TypeHint.RichText],
  map: coerceToString,
  debounceMs: 2000,
  minWidth: 200,
  grow: 1.0,
});
registerInterface("string.short", {
  hints: [TypeHint.Name, TypeHint.Uuid, TypeHint.Email, TypeHint.Url, TypeHint.Key],
  map: coerceToString,
  debounceMs: 500,
  minWidth: 150,
  grow: 0.5,
});
registerInterface("string.datetime", {
  hints: [TypeHint.Date, TypeHint.Datetime, TypeHint.Time],
  map: coerceToDatetime,
  debounceMs: 500,
  minWidth: 150,
  grow: 0.5,
});
registerInterface("secret", {
  tags: [TypeTag.String, TypeTag.Number],
  read: (t, v) => (isValidSecretRecord(v) ? v : null),
  isSecret: true,
  minWidth: 200,
  grow: 0.5,
});
// number
registerInterface("number", {
  tags: [TypeTag.Number],
  map: coerceToNumber,
  debounceMs: 1000,
  minWidth: 150,
  grow: 0.5,
});
registerInterface("number.rating", {
  hints: [TypeHint.Rating],
  map: coerceToNumber,
  minWidth: 120, // sync with RatingInterface max stars
  inline: true,
});
// boolean
registerInterface("boolean.checkbox", {
  tags: [TypeTag.Boolean],
  map: coerceToBoolean,
  minWidth: 80,
  inline: true,
});
registerInterface("boolean.toggle", {
  hints: [TypeHint.Toggle],
  map: coerceToBoolean,
  minWidth: 80,
  inline: true,
});
registerInterface("boolean.thumbs", {
  hints: [TypeHint.Thumbs],
  map: coerceToBoolean,
  minWidth: 80,
  inline: true,
});
// type reference
registerInterface("enum", {
  tags: [TypeTag.Enum],
  read: (t, v) => toArray(t, v),
  write: (t, v) => coerceToScalarOrArray(t, v),
  supportsList: true,
  minWidth: 200,
  grow: 1.0,
});
registerInterface("struct", {
  tags: [TypeTag.Struct],
  read: (t, v) => toArray(t, v),
  write: (t, v) => coerceToScalarOrArray(t, v),
  supportsList: true,
  minWidth: 200,
  grow: 1.0,
});
// blob
registerInterface("blob", {
  tags: [TypeTag.Blob],
  read: (t, v) => toArray(t, v).filter(isValidBlobRecord),
  write: (t, v) => coerceToScalarOrArray(t, v),
  supportsList: true,
  minWidth: 200,
  grow: 1.0,
});
// vector
registerInterface("vector", {
  tags: [TypeTag.Vector],
  read: (t, v) => v,
  write: (t, v) => v,
  supportsList: false,
  minWidth: 60,
  grow: 0.5,
  inline: true,
});
// node
registerInterface("node", {
  tags: [TypeTag.Node],
  read: (t, v) => toArray(t, v),
  write: (t, v) => coerceToScalarOrArray(t, v),
  supportsList: true,
  minWidth: 200,
  grow: 1.0,
});

export function getInputInterface(type: Field): ValueInterface | undefined {
  let filtered = Object.values(interfaces);
  // find most specific interface that supports the type
  filtered = filtered.filter((i) => {
    if (type.flags & TypeFlag.IS_ARRAY && !i.supportsList) {
      return false;
    }
    if (Boolean(type.flags & TypeFlag.IS_SECRET) != Boolean(i.isSecret)) {
      return false;
    }
    return true;
  });
  // prefer find by hint
  if (type.hint != null) {
    const byHint = filtered.find((i) => i.hints?.includes(type.hint!));
    if (byHint != null) {
      return byHint;
    }
  }
  // if hint not found fall back to tag
  return filtered.find((i) => i.tags?.includes(type.tag));
}

export function getMinWidth(type: Field): number | undefined {
  const iface = getInputInterface(type);
  return iface?.minWidth;
}

export function readValue(type: Field, value: any): any {
  // doesn't this belong more in typing?
  const iface = getInputInterface(type);
  if (iface?.read) {
    return iface.read(type, value);
  }
  return value;
}
