import { getPropertyType } from "@/language/field";
import { getPathKey } from "@/language/path";
import { packValue } from "@/language/value";
import {
  BlockData,
  ExpressionType,
  ObjectType,
  RunData,
  ActionData,
  UserData,
  type ExpressionData,
  type SortType,
  ComputedValueData,
  PathData,
  ComputedValueKind,
} from "@/proto/wire";
import { computed, Ref } from "vue";

export function makeExpression(
  options: { type: ExpressionType; value?: any } & Partial<ExpressionData>,
): ExpressionData {
  let valuePacked;
  if (options.value != null) {
    if (options.propertyPtr == null) throw new Error("propertyPtr required to pack Expression.value");
    let propertyType = getPropertyType(options.propertyPtr);
    if (options.type == ExpressionType.IN || options.type == ExpressionType.NOT_IN) {
      propertyType = { ...propertyType, isList: true }; // coerce type to list
    }
    valuePacked = packValue(options.value, propertyType);
  } else {
    valuePacked = options.valuePacked;
  }

  return {
    metatype: ObjectType.EXPRESSION,
    clauses: [],
    ...options,
    valuePacked,
  };
}

export function makeSort(
  options: Pick<ExpressionData, "sortMode" | "propertyPtr"> & { type: SortType },
): ExpressionData {
  return makeExpression({ ...(options as unknown as ExpressionData) });
}

export function makeAndConditional(clauses: ExpressionData[]): ExpressionData | undefined {
  if (clauses.length == 0) return undefined;
  else return makeExpression({ type: ExpressionType.AND, clauses });
}

export type EditSubject = UserData | RunData | BlockData | ActionData;

/** Controls a SourceNode.computedValues */
export function useComputedValues(options: {
  computedValues: Readonly<Ref<ComputedValueData[]>>;
  computedPrefix?: Readonly<Ref<PathData>>;
  update: (computedValues: ComputedValueData[]) => void;
}) {
  const computedValuesByKey: Ref<Record<string, ComputedValueData>> = computed(() => {
    const values = options.computedValues.value;
    return values.reduce(
      (acc, value) => {
        acc[getPathKey(value.targetPath!)] = value;
        return acc;
      },
      {} as Record<string, ComputedValueData>,
    );
  });

  /** Checks if there is an active computed value for a given path */
  function has(path: PathData | string | undefined): boolean {
    if (path == null) return false;
    if (typeof path != "string") path = getPathKey(path);
    return computedValuesByKey.value[path]?.isActive ?? false;
  }

  /** Gets the active computed value for a given path */
  function get(path: PathData | string | undefined): ComputedValueData | undefined {
    if (path == null) return undefined;
    if (typeof path != "string") path = getPathKey(path);
    return computedValuesByKey.value[path];
  }

  /** Clears any computed value for a given path */
  function clear(path: PathData | string | undefined): void {
    if (path == null) return;
    if (typeof path != "string") path = getPathKey(path);
    const newComputedValues = options.computedValues.value.filter((cv) => getPathKey(cv.targetPath!) != path);
    options.update(newComputedValues);
  }

  /** Sets an active computed value for a given path */
  function set(path: PathData, value?: ComputedValueData): void {
    const computedValue = {
      ...value,
      metatype: ObjectType.COMPUTED_VALUE,
      kind: ComputedValueKind.PATH,
      targetPath: path,
      isActive: true,
    };
    const pathKey = getPathKey(path);
    options.update([
      ...(options.computedValues.value.filter((cv) => cv.targetPath != null && getPathKey(cv.targetPath) != pathKey) ??
        []),
      computedValue,
    ]);
  }

  /** Toggles an active computed value for a given path */
  function toggle(path: PathData): void {
    if (has(path)) {
      clear(path);
    } else {
      set(path);
    }
  }

  return { computedValues: options.computedValues, computedValuesByKey, has, get, clear, set, toggle };
}
