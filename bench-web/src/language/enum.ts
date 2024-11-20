import { ENUM_TYPES, ENUM_TITLE_BY_TYPE, FILTERED_ENUMS, toCamelName } from "@/language/const";
import { EnumType, IconData, EnumTypeMapping, ENUM_BY_TYPE } from "@/proto/wire";
import { ICONS_BY_ENUM_TYPE } from "@/ui/icon";
import { toCasing, Casing } from "@/utils/string";

export type EnumOption<T extends EnumType = EnumType> = {
  id: string;
  icon?: IconData;
  title: string;
  value: EnumTypeMapping[T];
  isHidden?: boolean;
};

const ENUM_OPTIONS_BY_TYPE: Record<EnumType, EnumOption[]> = Object.fromEntries(
  ENUM_TYPES.map((enumType) => [enumType, makeEnumOptions(enumType)]),
) as Record<EnumType, EnumOption[]>;

function makeEnumOptions<T extends EnumType>(enumType: T): EnumOption<T>[] {
  const protoEnum = ENUM_BY_TYPE[enumType];
  const icons = ICONS_BY_ENUM_TYPE[enumType];
  const titles = ENUM_TITLE_BY_TYPE[enumType];
  const availableEnums =
    FILTERED_ENUMS[enumType] ?? Object.values(protoEnum).filter((v) => typeof v == "number" && v > 0);
  const options: EnumOption<T>[] = availableEnums.map((value) => {
    const icon = icons?.[value];
    const name = protoEnum[value] as string;
    if (name == null) throw new Error(`missing enum option ${value} in ${EnumType[enumType]}`);
    const title = titles?.[value] ?? toCasing(name, Casing.CAMEL, true);
    const option: EnumOption<T> = { id: value.toString(), icon, title, value: value as EnumTypeMapping[T] };
    return option;
  });
  return options;
}

export function getEnumOptions<T extends EnumType>(enumType: T): EnumOption<T>[] {
  return ENUM_OPTIONS_BY_TYPE[enumType] as EnumOption<T>[];
}

export function getEnumTitle<T extends EnumType>(enumType: T, enumValue: EnumTypeMapping[T]): string {
  return ENUM_TITLE_BY_TYPE[enumType]?.[enumValue] ?? toCamelName(ENUM_BY_TYPE[enumType], enumValue);
}
