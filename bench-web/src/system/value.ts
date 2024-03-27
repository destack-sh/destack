import { StructType, type TypeInfoData } from "@/proto/wire";
import { makeDefaultStruct } from "@/proto/wiring";

export function makeTypeInfo(partial: Partial<Omit<TypeInfoData, "metatype">>): TypeInfoData {
  return makeDefaultStruct({ metatype: StructType.TYPE_INFO, ...partial });
}
