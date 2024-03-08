import { IconKind, type IconData, BenchType } from "@/proto/wire";
import type { FunctionalComponent } from "vue";

export const IconInline: FunctionalComponent<Pick<IconData, "emoji" | "file" | "name">> = (props, context) => {
  if (props.name) {
    // font awesome
    return <i class={props.name} />;
  } else if (props.emoji) {
    return <span>{props.emoji}</span>;
  } else {
    throw new Error(`unexpected icon ${props}`);
  }
};

export function makeIcon(icon: Pick<IconData, "emoji" | "file" | "name">): IconData {
  let kind: IconKind;
  if (icon.emoji) {
    kind = IconKind.EMOJI;
  } else if (icon.file) {
    kind = IconKind.FILE;
  } else if (icon.name) {
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