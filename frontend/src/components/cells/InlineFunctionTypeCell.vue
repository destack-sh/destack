<script lang="ts" setup>
import { useStatementContext } from "@/components/statement";
import { ArrowLongRightIcon } from "@heroicons/vue/24/outline";
import { computed, ref, type Ref } from "vue";

const context = useStatementContext();

const addInputRef: Ref<HTMLButtonElement | null> = ref(null);
const addOutputRef: Ref<HTMLButtonElement | null> = ref(null);

const hasOutput = computed(() => false);

function insertInput() {
  console.log("insert input");
}

function insertOutput() {
  console.log("insert output");
}
</script>
<template>
  <div class="flex flex-row">
    <button
      ref="addInputRef"
      tabindex="-1"
      v-if="context.focused.value"
      @click="insertInput"
      class="w-fit rounded-sm px-0.5 text-gray-400 hover:bg-orange-50 hover:text-gray-700"
    >
      +input
    </button>
    <button
      ref="addOutputRef"
      v-if="context.focused.value"
      tabindex="-1"
      @click="insertOutput"
      class="inline-flex w-fit items-center rounded-sm px-0.5 text-gray-400 hover:bg-orange-50 hover:text-gray-700"
    >
      <ArrowLongRightIcon v-if="!hasOutput" class="mr-2 h-4 w-4" />
      +output
    </button>
  </div>
</template>
