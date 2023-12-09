<script lang="ts" setup>
import ConditionalTile from "@/components/tiles/ConditionalTile.vue";
import type { Conditional } from "@/gql/graphql";
import type { Field, Statement } from "@/state/module";
import { PlusIcon } from "@heroicons/vue/24/solid";
import { computed } from "vue";

const props = defineProps<{ fields: Field[]; modelValue?: Conditional[] }>();
const emit = defineEmits<{ (e: "update:modelValue", value?: Conditional[]): void }>();

const fieldsByCk = computed(() => {
  const map: Record<string, Field> = {};
  for (const field of props.fields) {
    map[field.ck] = field;
  }
  return map;
});

function updateConditional(index: number, conditional?: Conditional) {
  const value = [...(props.modelValue ?? [])];
  if (conditional == null) {
    value.splice(index, 1);
  } else {
    value[index] = conditional;
  }
  emit("update:modelValue", value);
}
</script>
<template>
  <div>
    <!-- nocheckin filter -->
    <ConditionalTile
      v-for="(conditional, i) in modelValue?.filter((c) => fieldsByCk[c.field as string] != null) ?? []"
      :key="i"
      :field="fieldsByCk[conditional.field as string]"
      :modelValue="conditional"
      @update:modelValue="($event) => updateConditional(i, $event)"
    />
    <button class="flex flex-row items-center rounded-sm px-0.5 py-0.5 text-gray-400 hover:bg-amber-100">
      <!-- nocheckin: select field to filter -->
      <PlusIcon class="h-4 w-4" />
      Filter
    </button>
  </div>
</template>
