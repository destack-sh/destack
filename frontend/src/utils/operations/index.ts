import { useFileOps } from "@/utils/operations/file";
import { useSymbolOps } from "@/utils/operations/symbol";
import { useCompilationOps } from "@/utils/operations/compilation";
import { useProjectVersionOps } from "@/utils/operations/version";
import { defineStore } from "pinia";

type Operation = {
  id?: string;
  type: string;
  apply(): Promise<void>;
  undo?(): Promise<void>;
};

export function useOperations() {
  return {
    file: useFileOps(),
    symbol: useSymbolOps(),
    compilation: useCompilationOps(),
    version: useProjectVersionOps(),
  };
}

export const useOperationsStore = defineStore("operations", {
  state: () => ({
    inflight: [] as Operation[],
    undoStack: [] as Operation[],
    redoStack: [] as Operation[],
  }),
  getters: {
    canUndo(state): boolean {
      return state.undoStack.length > 0;
    },
    canRedo(state): boolean {
      return state.redoStack.length > 0;
    },
  },
  actions: {
    async perform(operation: Operation): Promise<void> {
      operation = { ...operation, id: operation.id ?? Math.random().toString(16) };
      console.log(`perform ${operation.type} (id=${operation.id})`);
      this.inflight.push(operation);
      await operation.apply();
      this.inflight = this.inflight.filter((op) => op.id !== operation.id);
      if (operation.undo != null) {
        this.undoStack.push(operation);
      }
      this.redoStack = []; // reset redo stack
    },
    async undo(): Promise<void> {
      const operation = this.undoStack.pop();
      if (operation == null || operation.undo == null) {
        return;
      }
      console.log(`undo ${operation.type}`);
      await operation.undo();
      this.redoStack.push(operation);
    },
    async redo(): Promise<void> {
      const operation = this.redoStack.pop();
      if (operation == null) {
        return;
      }
      console.log(`redo ${operation.type}`);
      await operation.apply();
      this.undoStack.push(operation);
    },
  },
});
