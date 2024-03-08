import type { NodeReferenceData, ViewData } from "@/proto/wire";

export type ViewPropsFlags = Partial<Pick<ViewData, "isVisible" | "isDisabled" | "isLoading" | "isInput" | "isSecret">>;

export type ViewPropsCommon = Pick<
  ViewData,
  "type" | "name" | "title" | "text" | "icon" | "valuePacked" | "nodePtr" | "variant"
>;

export type ViewProps = ViewPropsCommon &
  ViewPropsFlags & {
    self?: NodeReferenceData | undefined | null;
  };

export type ViewPropsAnchored = ViewProps & {
  self: NodeReferenceData;
};

export type ViewEmits = {
  (e: "navigateLeft", node: ViewData): void;
};
