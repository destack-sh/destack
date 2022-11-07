import type { Taggable } from "@/types/tags";

export type Flow = Taggable & {
  id: string;
  name: string;
  description?: string;
  versions: string[]; // fk to FlowVersion.id
  head?: FlowVersion;
  created_at: string;
};

export type FlowVersion = Taggable & {
  id: string;
  name: string;
  version: string;
  parents: string[]; // fk to FlowVersion.version
  description?: string;
  flow: string; // fk to Flow.name
  created_at: string;
  committed: boolean;
};

export type FlowInstruction = Taggable & {
  id: string;
  name: string;
};

export type FlowRuntimeData = {
  // inputs by node name
  inputs?: Record<string, Record<string, any>>;
};

export type FlowRuntimeValidation = "off" | "lazy" | "full";

export type FlowExecutionOptions = {
  blocking: boolean;
  validate: FlowRuntimeValidation;
  captured_connection_types?: Array<"input" | "argument">;
  captured_edges?: string[];
};
