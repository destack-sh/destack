<script lang="ts" setup>
import { NodeType } from "@/proto/wire";
import { SomeNodeReferenceData, TypedNodeReferenceData } from "@/proto/wiring";
import { RunTree } from "@/system/runtime";
import { pkgGraph } from "@/system/space";
import { Ref, toRef } from "vue";

const props = defineProps<{ nodePtr: SomeNodeReferenceData }>();
const nodePtr = toRef(props, "nodePtr") as Ref<TypedNodeReferenceData<NodeType.RUN> | null>;

//
// Run
//

const runTree = new RunTree(pkgGraph, nodePtr);
const run = runTree.runRef;

//
// Spans / Events
//

// nocheckin: RunTimeline
</script>
<template>
  <div>
    RunTimeline with {{ runTree.runs.length }} runs
    <div>
      <div v-for="run in runTree.runs" :key="run.id">
        {{ run.id }}
        {{ run.duration?.seconds }} {{ run.duration?.nanos }}
      </div>
    </div>
  </div>
</template>
