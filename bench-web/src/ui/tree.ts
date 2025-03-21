import { declareCommands } from "@/ui/command";

// tree
declareCommands<"tree">({
  // create
  "tree.create.above": {
    icon: "fas fa-arrow-up",
    title: "Create Above",
    text: "Create a new item above this item",
  },
  "tree.create.below": {
    icon: "fas fa-arrow-down",
    title: "Create Below",
    text: "Create a new item below this item",
  },
  "tree.create.inside": {
    icon: "fas fa-arrow-right",
    title: "Create Inside",
    text: "Create a new item inside this item",
  },
});
