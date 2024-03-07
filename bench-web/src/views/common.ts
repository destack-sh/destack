import type { NodeReferenceData, ViewData } from "@/proto/wire";

export type ViewFlags = "isVisible" | "isDisabled" | "isLoading" | "isInput" | "isSecret";

export type ViewProps = Pick<ViewData, "type" | "name" | "title" | "text" | "icon" | "nodePtr" | "valuePacked" | ViewFlags> & {
  self: NodeReferenceData | undefined | null;
};

export type ViewPropsAnchored = ViewProps & {
  self: NodeReferenceData;
};

export type ViewEmits = {
  (e: "navigateLeft", node: ViewData): void;
};
