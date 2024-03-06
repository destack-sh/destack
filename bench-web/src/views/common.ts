import type { NodeReferenceData, ViewData } from "@/proto/wire";

export type ViewFlags = "isVisible" | "isDisabled" | "isLoading" | "isInput" | "isSecret";

export type ViewProps = Pick<ViewData, "type" | "name" | "title" | "text" | "icon" | "valuePacked" | ViewFlags> & {
  node: NodeReferenceData | null;
};

export type ViewEmits = {
  (e: "navigateLeft", node: ViewData): void;
};
