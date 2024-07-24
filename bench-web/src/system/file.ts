import { FileType, ViewType } from "@/proto/wire";

export const FILE_TYPE_BY_VIEW_TYPE: Partial<Record<ViewType, FileType>> = {
  [ViewType.IMAGE]: FileType.IMAGE,
  [ViewType.AUDIO]: FileType.AUDIO,
  [ViewType.VIDEO]: FileType.VIDEO,
  [ViewType.DOCUMENT]: FileType.DOCUMENT,
};
