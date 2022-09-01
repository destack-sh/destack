import {
  isFieldSpec,
  isFieldTypePrimitive,
  type AnySpec,
  type FieldSpec,
  type FieldType,
  type FieldTypePrimitive,
  type RecordType,
} from "@/types";

export function unravelSpec(spec: AnySpec | AnySpec[]): AnySpec[] {
  if (Array.isArray(spec)) {
    return spec.flatMap(unravelSpec);
  } else if (isFieldSpec(spec)) {
    if (isFieldTypePrimitive(spec.type)) {
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

export function filterToFieldSpecs(spec: AnySpec[]): FieldSpec[] {
  return unravelSpec(spec).filter(isFieldSpec) as FieldSpec[];
}

export function flatMapFieldSpecs(spec: AnySpec[]): FieldSpec[] {
  return unravelSpec(spec)
    .filter((spec) => isFieldSpec(spec))
    .flatMap((spec) => unravelSpec(spec as FieldSpec) as FieldSpec[]);
}

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

  if (isFieldTypePrimitive(part)) {
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
