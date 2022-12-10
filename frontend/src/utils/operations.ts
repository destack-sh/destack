import { defineStore } from "pinia";

type Operation = {
  type: string;
  apply(): Promise<void>;
  undo(): Promise<void>;
};

export const useOperationsStore = defineStore("operations", {
  state: () => ({
    undoStack: [] as Operation[],
    redoStack: [] as Operation[],
  }),
  actions: {
    async perform(operation: Operation): Promise<void> {
      console.log(`perform ${operation.type}`);
      await operation.apply();
      this.undoStack.push(operation);
      this.redoStack = [];
    },
    async undo(): Promise<void> {
      const operation = this.undoStack.pop();
      if (operation == null) {
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
