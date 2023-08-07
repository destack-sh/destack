<script lang="ts" setup>
import { useAppearance } from "@/state/appearance";
import { useCurrentModule } from "@/state/module";
import { useOperations } from "@/state/operations";
import { useStatementContext } from "@/state/statement";
import { ClockIcon } from "@heroicons/vue/24/solid";
import { ref } from "vue";

const context = useStatementContext();
const module = useCurrentModule();
const appearance = useAppearance();
const ops = useOperations();

const adding = ref(false);
</script>
<template>
  <div class="group relative flex flex-row gap-1.5">
    <!-- TODO nocheckin remove: ultra hack to show fake trigger -->
    <span
      v-if="context.statement.value.name == 'load from github'"
      class="flex flex-row items-center rounded-xl bg-orange-100 px-1.5 text-orange-900 ring-1 ring-inset ring-orange-600/20"
    >
      <ClockIcon class="mr-1 h-4 w-4" />
      <span class="text-sm">every 1h</span>
    </span>
  </div>
</template>
