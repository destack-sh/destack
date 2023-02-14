import { useFileOps } from "@/state/operations/file";
import { useRuntimeOps } from "@/state/operations/runtime";
import { useStatementOps } from "@/state/operations/statement";
import { useSymbolContentOps } from "@/state/operations/symbol";
import { useProjectVersionOps } from "@/state/operations/version";
import { DateTime } from "luxon";
import { defineStore } from "pinia";
import { ref, type Ref } from "vue";

type Operation<T> = {
  id?: string;
  type: string;
  startedAt?: DateTime;
  key?: string | Record<string, string>;
  do(): Promise<T>;
  ret?: T;
  undo?(ret: T): Promise<void>;
};

const COMPLETED_STACK_SIZE = 500;
const STALE_TIME_SECONDS = 1;

// keep reactive now (can't use useNow because it attaches to component)
const now: Ref<DateTime> = ref(DateTime.now());
setInterval(() => (now.value = DateTime.now()), 100);

export const useOperationsStore = defineStore("operations", {
  state: () => ({
    inflight: [] as Operation<unknown>[],
    completed: [] as Operation<unknown>[],
    undoStack: [] as Operation<unknown>[],
    redoStack: [] as Operation<unknown>[],
  }),
  getters: {
    inflightLike(state): (type: string, key?: string) => Operation<unknown> | undefined {
      return (type, key) => state.inflight.filter((op) => op.type === type && (!key || op.key === key))[0];
    },
    hasInflightLike(state): (type: string, key?: string) => boolean {
      return (type, key) => state.inflight.some((op) => op.type === type && (!key || op.key === key));
    },
    hasInflight(state): boolean {
      return state.inflight.length > 0;
    },
    inflightStale(state): Operation<unknown>[] {
      const oneSecondAgo = now.value.minus({ seconds: STALE_TIME_SECONDS });
      return state.inflight.filter((op) => (op.startedAt as DateTime) < oneSecondAgo);
    },
    hasInflightStale(state): boolean {
      return this.inflightStale.length > 0;
    },
    completedLike(state): (type: string, key?: string) => Operation<unknown> {
      return (type, key) => state.completed.filter((op) => op.type === type && (!key || op.key === key))[0];
    },
    hasCompletedLike(state): (type: string, key?: string) => boolean {
      return (type, key) => state.completed.some((op) => op.type === type && (!key || op.key === key));
    },
    canUndo(state): boolean {
      return state.undoStack.length > 0;
    },
    canRedo(state): boolean {
      return state.redoStack.length > 0;
    },
  },
  actions: {
    reset(): void {
      this.$reset();
    },

    async _do<T>(operation: Operation<T>): Promise<T> {
      operation = { ...operation, startedAt: DateTime.now() };
      this.inflight.push(operation);
      const ret = await operation.do();
      operation.ret = ret;
      this.inflight = this.inflight.filter((op) => op.id !== operation.id);
      this.completed.push({ ...operation });
      // trim completed stack
      if (this.completed.length > COMPLETED_STACK_SIZE) {
        this.completed = this.completed.slice(this.completed.length - COMPLETED_STACK_SIZE);
      }
      return ret;
    },

    async perform<T>(operation: Operation<T>): Promise<T> {
      operation = { ...operation, id: operation.id ?? Math.random().toString(16).substring(2, 8) };
      console.log(`perform ${operation.type} (id=${operation.id})`);

      // enable undo even before the operation is performed (for responsiveness)
      if (operation.undo != null) {
        this.undoStack.push(operation);
      }
      const ret = await this._do(operation);
      this.redoStack = []; // reset redo stack, maybe store a redo branch backup?
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
      await this._do(operation);
      this.undoStack.push(operation);
    },
  },
});

export function useOperations() {
  return {
    file: useFileOps(),
    content: useSymbolContentOps(),
    statement: useStatementOps(),
    symbol: useSymbolContentOps(),
    runtime: useRuntimeOps(),
    version: useProjectVersionOps(),
  };
}
