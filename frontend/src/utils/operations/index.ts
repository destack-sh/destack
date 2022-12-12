import { useFileOps } from "@/utils/operations/file";
import { useSymbolOps } from "@/utils/operations/symbol";
import { useCompilationOps } from "@/utils/operations/compilation";
import { useProjectVersionOps } from "@/utils/operations/version";
import { defineStore } from "pinia";

type Operation<T> = {
  id?: string;
  type: string;
  do(): Promise<T>;
  ret?: T;
  undo?(ret: T): Promise<void>;
};

export const useOperationsStore = defineStore("operations", {
  state: () => ({
    inflight: [] as Operation<unknown>[],
    undoStack: [] as Operation<unknown>[],
    redoStack: [] as Operation<unknown>[],
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
    async perform<T>(operation: Operation<T>): Promise<T> {
      operation = { ...operation, id: operation.id ?? Math.random().toString(16) };
      console.log(`perform ${operation.type} (id=${operation.id})`);

      this.inflight.push(operation);
      const ret = await operation.do();
      operation.ret = ret;

      this.inflight = this.inflight.filter((op) => op.id !== operation.id);
      if (operation.undo != null) {
        this.undoStack.push(operation);
      }
      this.redoStack = []; // reset redo stack
      return ret;
    },

    async undo(): Promise<void> {
      const operation = this.undoStack.pop();
      if (operation == null || operation.undo == null) {
        return;
      }
      console.log(`undo ${operation.type} (id=${operation.id})`);
      await operation.undo(operation.ret);
      this.redoStack.push(operation);
    },

    async redo(): Promise<void> {
      const operation = this.redoStack.pop();
      if (operation == null) {
        return;
      }
      console.log(`redo ${operation.type} (id=${operation.id})`);
      await operation.do();
      this.undoStack.push(operation);
    },
  },
});

export function useOperations() {
  return {
    file: useFileOps(),
    symbol: useSymbolOps(),
    compilation: useCompilationOps(),
    version: useProjectVersionOps(),
  };
}
