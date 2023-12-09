<script lang="ts" setup>
import SortTile from "@/components/tiles/SortTile.vue";
import type { Sort } from "@/gql/graphql";
import type { Field } from "@/state/module";
import { PlusIcon } from "@heroicons/vue/24/solid";
import { computed } from "vue";

const props = defineProps<{
  fields: Field[];
  modelValue?: Sort[];
}>();
const emit = defineEmits<{
  (e: "update:modelValue", value?: Sort[]): void;
}>();

const fieldsByCk = computed(() => {
  const map: Record<string, Field> = {};
  for (const field of props.fields) {
    map[field.ck] = field;
  }
  return map;
});

function updateSort(index: number, sort?: Sort) {
  const value = [...(props.modelValue ?? [])];
  if (sort == null) {
    value.splice(index, 1);
  } else {
    value[index] = sort;
  }
  emit("update:modelValue", value);
}
</script>
<template>
  <div class="flex flex-row gap-1">
    <!-- nocheckin sort -->
    <SortTile
      v-for="(sort, i) in modelValue?.filter((s) => fieldsByCk[s.field] != null) ?? []"
      :key="i"
      :field="fieldsByCk[sort.field]"
      :modelValue="sort"
      @update:modelValue="($event) => updateSort(i, $event)"
    />
    <button class="flex flex-row items-center rounded-sm px-0.5 py-0.5 text-gray-400 hover:bg-amber-100">
      <!-- nocheckin: select field to sort -->
      <PlusIcon class="h-4 w-4" />
      Sort
    </button>
  </div>
</template>
