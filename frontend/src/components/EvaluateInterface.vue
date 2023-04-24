<script lang="ts" setup>
import type { SymbolType } from "@/gql/graphql";
import { useAppearance } from "@/state/appearance";
import { EDITOR_INTERFACE_STATE, type EditorInterfaceState } from "@/state/editor";
import { symbolOf, useCurrentInterpModule } from "@/state/runtime";
import { computed, inject } from "vue";

const props = defineProps<{ symbolId: string; symbolType: SymbolType; focused: boolean }>();
const symbol = computed(() => symbolOf(props.symbolId));

const state = inject<EditorInterfaceState>(EDITOR_INTERFACE_STATE);
if (state == null) {
  throw new Error("need interface state context");
}

const interp = useCurrentInterpModule();

const appearance = useAppearance();
</script>
<template>
  <div
    class="flex flex-col bg-white px-12 py-6"
    :class="{ 'font-mono': appearance.fontMono, 'text-sm': appearance.textSmall, 'text-md': !appearance.textSmall }"
  >
    <!-- Header -->
    <div class="mx-auto w-full max-w-[800px]">
      <h2 class="flex flex-row items-baseline gap-1">
        <!-- the runnable should maybe be configurable, but that would require mutating the editor instance -->
        <span class="text-3xl font-bold text-gray-900">evaluate: {{ symbol?.name }}</span>
      </h2>
      <!-- TODO @Feature: Core metrics -->
    </div>
    <!-- Samples -->
    <!-- given samples -->
    <!-- generated samples -->
  </div>
</template>
