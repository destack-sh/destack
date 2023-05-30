import type { SimpleType } from "@/state/statement";
import { TypeHint, TypeTag } from "@/gql/graphql";
import { isValidObjectRecord } from "@/state/object";
import { TypeFlag } from "@/state/runtime";
import { isValidSecretRecord } from "@/state/secret";

export type ValueInterface = {
  id: string;
  tags?: TypeTag[];
  hints?: TypeHint[];
  debounceMs?: number;
  supportsList?: boolean;
  isSecret?: boolean;
  // read/write mapping
  read?(type: SimpleType, value: any): any;
  write?(type: SimpleType, value: any): any;
  map?(type: SimpleType, value: any): any;
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

// We automatically coerce to/from arrays as needed so we can smoothly
//  switch between array and non-array types without having to store everything
//  as an array upfront.  :ArrayCoercion

function fromArray(value: any) {
  if (Array.isArray(value)) {
    return value[0];
  } else {
    return value;
  }
}

function toArrayAsFlagged(type: SimpleType, value: any) {
  if (Array.isArray(value)) {
    if (!(type.flags & TypeFlag.IsArray)) {
      return value.slice(0, 1);
    } else {
      return value;
    }
  } else if (value != null) {
    return [value];
  }
  return [];
}

function toArrayIfFlagged(type: SimpleType, value: any) {
  if (type.flags & TypeFlag.IsArray) {
    if (Array.isArray(value)) {
      return value;
    } else {
      return [value];
    }
  } else {
    return fromArray(value);
  }
}

function coerceToBoolean(type: SimpleType, value: any) {
  value = fromArray(value);
  return typeof value == "boolean" ? value : false;
}

function coerceToString(type: SimpleType, value: any) {
  value = fromArray(value);
  if (typeof value == "number") {
    value = value.toString();
  }
  return typeof value == "string" ? value : "";
}

function coerceToNumber(type: SimpleType, value: any) {
  value = fromArray(value);
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
registerInterface("string.short", {
  hints: [TypeHint.Name, TypeHint.Uuid, TypeHint.Email, TypeHint.Url, TypeHint.Key],
  map: coerceToString,
  debounceMs: 1000,
  minWidth: 200,
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
  minWidth: 40,
  inline: true,
});
registerInterface("boolean.toggle", {
  hints: [TypeHint.Toggle],
  map: coerceToBoolean,
  minWidth: 40,
  inline: true,
});
registerInterface("boolean.thumbs", {
  hints: [TypeHint.Thumbs],
  map: coerceToBoolean,
  minWidth: 40,
  inline: true,
});
// other
registerInterface("enum", {
  tags: [TypeTag.Enum],
  read: (t, v) => toArrayAsFlagged(t, v),
  write: (t, v) => toArrayIfFlagged(t, v),
  supportsList: true,
  minWidth: 200,
  grow: 1.0,
});
registerInterface("struct", {
  tags: [TypeTag.Struct],
  read: (t, v) => toArrayAsFlagged(t, v),
  write: (t, v) => toArrayIfFlagged(t, v),
  supportsList: true,
  minWidth: 200,
  grow: 1.0,
});
registerInterface("file", {
  tags: [TypeTag.File, TypeTag.Audio, TypeTag.Image, TypeTag.Video],
  read: (t, v) => toArrayAsFlagged(t, v).filter(isValidObjectRecord),
  write: (t, v) => toArrayIfFlagged(t, v),
  supportsList: true,
  minWidth: 200,
  grow: 1.0,
});

export function getInterface(type: SimpleType): ValueInterface | undefined {
  let filtered = Object.values(interfaces);
  // find most specific interface that supports the type
  filtered = filtered.filter((i) => {
    if (type.flags & TypeFlag.IsArray && !i.supportsList) {
      return false;
    }
    if (Boolean(type.flags & TypeFlag.IsSecret) != Boolean(i.isSecret)) {
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

export function getMinWidth(type: SimpleType): number | undefined {
  const iface = getInterface(type);
  return iface?.minWidth;
}
