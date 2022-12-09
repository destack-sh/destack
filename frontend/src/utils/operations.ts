import { defineStore } from "pinia";

type Operation = {
  id: string;
};

const operationStore = defineStore("operations", {
  state: () => ({
    undoStack: [] as Operation[],
    redoStack: [] as Operation[],
  }),
});
