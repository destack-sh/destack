import { IconKind, type IconData, BenchType } from "@/proto/wire";
import type { FunctionalComponent } from "vue";

export const IconInline: FunctionalComponent<Pick<IconData, "emoji" | "file" | "faName">> = (props) => {
  if (props.faName) {
    // font awesome
    return <i class={props.faName + " w-[18px] text-center"} />;
  } else if (props.emoji) {
    return <span>{props.emoji}</span>;
  } else {
    throw new Error(`unexpected icon ${props}`);
  }
};

type IconIn = string | Pick<IconData, "emoji" | "file" | "faName">;
export function makeIcon(icon: IconIn): IconData {
  let kind: IconKind;
  if (typeof icon == "string") {
    return { metatype: BenchType.ICON, kind: IconKind.FONT_AWESOME, faName: icon, setProperties: [] };
  } else if (icon.emoji) {
    kind = IconKind.EMOJI;
  } else if (icon.file) {
    kind = IconKind.FILE;
  } else if (icon.faName) {
    kind = IconKind.FONT_AWESOME;
  } else {
    throw new Error(`unexpected icon ${icon}`);
  }
  return {
    metatype: BenchType.ICON,
    kind,
    ...icon,
    setProperties: [],
  };
}

export function toIconMaybe(icon?: IconIn | null): IconData | undefined {
  if (icon == null) return undefined;
  return makeIcon(icon);
}
