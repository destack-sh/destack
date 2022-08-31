import {
  isFieldSpec,
  isFieldType,
  type FieldSpec,
  type FieldType,
  type FieldTypePrimitive,
  type RecordType,
} from "@/types";

export function unwrapFieldSpec(maybeFieldSpec: FieldType) {
  if (isFieldSpec(maybeFieldSpec)) {
    return (maybeFieldSpec as FieldSpec).type;
  } else {
    return maybeFieldSpec;
  }
}

export function fieldTypeContains(whole: FieldTypePrimitive, part: FieldTypePrimitive) {
  if (part._type == "ValueType") {
    return whole._type == "ValueType" && part.dtype == whole.dtype;
  } else if (part._type == "EnumType") {
    return whole == part;
  } else {
    throw new Error("unknown field type: " + JSON.stringify(part));
  }
}

export function specContains(whole: RecordType, part: RecordType): boolean {
  /**
   * Checks if the part is contained in the whole
   */

  whole = unwrapFieldSpec(whole);
  part = unwrapFieldSpec(part);

  if (isFieldType(part)) {
    return fieldTypeContains(whole as FieldTypePrimitive, part as FieldTypePrimitive);
  } else if (Array.isArray(part)) {
    return Array.isArray(whole) && specContains(whole[0], part[0]);
  } else {
    if (typeof whole != "object") {
      return false;
    }

    whole = whole as Record<string, FieldSpec>;
    part = part as Record<string, FieldSpec>;
    for (const key in part) {
      if (whole[key] == null) {
        return false;
      }
      if (!specContains(part[key], whole[key])) {
        return false;
      }
    }
  }

  return true;
}
