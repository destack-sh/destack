<script lang="ts" setup>
import { supervisor } from "@/proto/services";
import { NodeType } from "@/proto/wire";
import Windowed from "@/views/containers/Windowed.vue";
import Bar from "@/views/intrinsics/Bar.vue";

// noheckin: testing supervisor
const numBenches = supervisor.aggregateNodes({
  bases: [],
  sort: [],
  nodeType: NodeType.USER,
});
</script>
<template>
  <div class="w-full">
    <Bar />
    <Windowed />
    <!-- {{ numBenches.response.then((res) => res)}} -->
  </div>
</template>
