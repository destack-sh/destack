import { ENUM_TYPES, ENUM_TITLE_BY_TYPE, FILTERED_ENUMS, toCamelName } from "@/language/core/const";
import { EnumType, IconData, EnumTypeMapping, ENUM_BY_TYPE, ColorType, ENUM_OPTION_INFO_BY_TYPE } from "@/proto/wire";
import { makeIcon } from "@/ui/icon";
import { toCasing, Casing } from "@/utils/string";

export type EnumOption<T extends EnumType = EnumType> = {
  id: string;
  icon?: IconData;
  title: string;
  text?: string;
  color?: ColorType;
  value: EnumTypeMapping[T];
  isHidden?: boolean;
};

export const ENUM_OPTIONS_BY_TYPE: Record<EnumType, EnumOption[]> = Object.fromEntries(
  ENUM_TYPES.map((enumType) => [enumType, makeEnumOptions(enumType)]),
) as Record<EnumType, EnumOption[]>;
export const ENUM_OPTIONS_BY_VALUE: Record<EnumType, Record<number, EnumOption>> = Object.fromEntries(
  ENUM_TYPES.map((enumType) => [enumType, Object.fromEntries(makeEnumOptions(enumType).map((o) => [o.value, o]))]),
) as Record<EnumType, Record<number, EnumOption>>;

/** Get the EnumOptions for an enum type */
function makeEnumOptions<T extends EnumType>(enumType: T): EnumOption<T>[] {
  const protoEnum = ENUM_BY_TYPE[enumType];
  const icons = ENUM_OPTION_INFO_BY_TYPE[enumType];
  const titles = ENUM_TITLE_BY_TYPE[enumType];
  const availableEnums =
    FILTERED_ENUMS[enumType] ?? Object.values(protoEnum).filter((v) => typeof v == "number" && v > 0);
  const options: EnumOption<T>[] = availableEnums.map((value) => {
    const icon = icons?.[value]?.icon != null ? makeIcon(icons[value].icon) : undefined;
    const name = protoEnum[value] as string;
    if (name == null) throw new Error(`missing enum option ${value} in ${EnumType[enumType]}`);
    const title = titles?.[value] ?? toCasing(name, Casing.CAMEL, true);
    const color = ENUM_OPTION_INFO_BY_TYPE[enumType]?.[value]?.color;
    const text = ENUM_OPTION_INFO_BY_TYPE[enumType]?.[value]?.text;
    const option: EnumOption<T> = {
      id: value.toString(),
      icon,
      title,
      color,
      text,
      value: value as EnumTypeMapping[T],
    };
    return option;
  });
  return options;
}

/** Get the EnumOptions for an enum type */
export function getEnumOptions<T extends EnumType>(enumType: T): EnumOption<T>[] {
  return ENUM_OPTIONS_BY_TYPE[enumType] as EnumOption<T>[];
}

/** Get the EnumOption for an enum type and value */
export function getEnumOption<T extends EnumType>(enumType: T, enumValue: EnumTypeMapping[T]): EnumOption<T> | null {
  return (ENUM_OPTIONS_BY_VALUE[enumType]?.[enumValue as any] ?? null) as EnumOption<T> | null;
}

/** Get the title for an enum type and value */
export function getEnumTitle<T extends EnumType>(enumType: T, enumValue: EnumTypeMapping[T]): string {
  return ENUM_TITLE_BY_TYPE[enumType]?.[enumValue] ?? toCamelName(ENUM_BY_TYPE[enumType], enumValue);
}
