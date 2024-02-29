<script lang="ts" setup>
import { reactiveUnaryCall, supervisor } from "@/proto/services";
import { ExpressionOp, NodeType, StructType } from "@/proto/wire";
import { makeStruct } from "@/proto/wiring";
import Windowed from "@/views/containers/Windowed.vue";
import Bar from "@/views/intrinsics/Bar.vue";

// noheckin: testing supervisor
const { result, pending } = reactiveUnaryCall(supervisor, supervisor.aggregateNodes, {
  aggregation: makeStruct(StructType.EXPRESSION, {
    op: ExpressionOp.COUNT,
    clauses: [],
  }),
  bases: [],
  sort: [],
  nodeType: NodeType.BENCH,
});
</script>
<template>
  <div class="w-full">
    <Bar />
    <Windowed />
    {{ !!pending }}
    {{ result?.aggregation?.count }}
  </div>
</template>
