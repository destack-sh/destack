import type { NodeReferenceData, ViewData } from "@/proto/wire";

export type ViewPropsFlags = Partial<Pick<ViewData, "isVisible" | "isDisabled" | "isLoading" | "isInput" | "isSecret">>;
export type ViewPropsCommon = Pick<ViewData, "type" | "name" | "title" | "text" | "icon" | "valuePacked" | "nodePtr">;
export type ViewPropsStyle = Pick<ViewData, "variant" | "font">;
export type ViewPropsLayout = Pick<ViewData, "position" | "size" | "margin" | "padding" | "orientation" | "alignment">;
export type ViewPropsBase = ViewPropsCommon &
  ViewPropsFlags & {
    self?: NodeReferenceData | undefined | null;
  };
export type ViewPropsAnchoredBase = ViewPropsBase & {
  self: NodeReferenceData;
};
export type ViewPropsAll = ViewPropsBase & ViewPropsStyle & ViewPropsLayout;
export type ViewPropsAllAnchored = ViewPropsAnchoredBase & ViewPropsStyle & ViewPropsLayout;

export type ViewEmits = {
  (e: "navigateLeft", node: ViewData): void;
};
