import type { SimpleType } from "@/components/statement";
import { TypeHint, TypeTag } from "@/gql/graphql";
import { TypeFlag } from "@/state/runtime";

export type ValueInterface = {
  id: string;
  tags?: TypeTag[];
  hints?: TypeHint[];
  inline?: boolean;
  supportsList?: boolean;
  supportsSecret?: boolean;
  read?(type: SimpleType, value: any): any;
  write?(type: SimpleType, value: any): any;
  map?(type: SimpleType, value: any): any;
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
  return typeof value == "string" ? value : "";
}

function coerceToNumber(type: SimpleType, value: any) {
  value = fromArray(value);
  if (typeof value == "string") {
    value = Number.parseFloat(value.trim());
  }
  return typeof value == "number" ? value : null;
}

registerInterface("boolean.checkbox", {
  tags: [TypeTag.Boolean],
  map: coerceToBoolean,
  inline: true,
});
registerInterface("boolean.toggle", {
  tags: [],
  hints: [TypeHint.Toggle],
  map: coerceToBoolean,
  inline: true,
});
registerInterface("string", {
  tags: [TypeTag.String],
  map: coerceToString,
});
registerInterface("number", {
  tags: [TypeTag.Number],
  map: coerceToNumber,
});
registerInterface("enum", {
  tags: [TypeTag.Enum],
  read: (t, v) => toArrayAsFlagged(t, v),
  write: (t, v) => toArrayIfFlagged(t, v),
  supportsList: true,
});
registerInterface("file", {
  tags: [TypeTag.File, TypeTag.Audio, TypeTag.Image, TypeTag.Video],
  read: (t, v) => toArrayAsFlagged(t, v),
  write: (t, v) => toArrayIfFlagged(t, v),
  supportsList: true,
});

export function getInterface(type: SimpleType): ValueInterface | undefined {
  // find most specific interface that supports the type
  const withFlags = Object.values(interfaces).filter((i) => {
    if (type.flags & TypeFlag.IsArray && !i.supportsList) {
      return false;
    }
    if (type.flags & TypeFlag.IsSecret && !i.supportsSecret) {
      return false;
    }
    return true;
  });
  // prefer find by hint
  if (type.hint != null) {
    const byHint = withFlags.find((i) => i.hints?.includes(type.hint!));
    if (byHint != null) {
      return byHint;
    }
  }
  // if hint not found fall back to tag
  const byTag = withFlags.find((i) => i.tags?.includes(type.tag));
  return byTag;
}
