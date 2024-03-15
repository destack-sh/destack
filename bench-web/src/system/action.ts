import type { IconData, NodeReferenceData, TextData } from "@/proto/wire";
import type { KeymapSignature } from "@/utils/keymap";

export type ActionSource = { kind: "builtin"; id: string } | { kind: "block"; block: NodeReferenceData };

// TODO :Architecture: define Action as Struct so it can be provided by custom Views/...?
//  provide actions by tagging runnable (no args) Blocks with Action?
export type Action = {
  icon?: IconData;
  title: string;
  text?: TextData;
  shortcuts: KeymapSignature[];
  enabled?: boolean;
  source: ActionSource;
};
