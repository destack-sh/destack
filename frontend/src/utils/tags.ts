import type { Tag } from "@/types/tags";

export function getTagType(tag: Tag | string): string | undefined {
  if (typeof tag == "object") {
    tag = tag.name;
  }

  const typeSeparator = tag.indexOf(":");
  if (typeSeparator > 1) {
    return tag.slice(0, typeSeparator);
  } else {
    return undefined;
  }
}
