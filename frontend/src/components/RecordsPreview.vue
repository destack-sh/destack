<template>
  <table class="min-w-full border-none" style="border-spacing: 0">
    <!-- <thead class="bg-gray-50">
              <tr>
                <th
                  scope="col"
                  v-for="column in columns"
                  :key="column.name"
                  class="sticky top-0 z-10 border-b border-gray-300 bg-gray-50 bg-opacity-75 py-3.5 pl-4 pr-3 text-left text-sm font-semibold text-gray-900 backdrop-blur backdrop-filter sm:pl-6 lg:pl-8"
                >
                  {{ column.name }}}
                </th>
              </tr>
            </thead> -->
    <tbody class="bg-white">
      <tr v-for="(row, i) in rows" :key="i">
        <td
          v-for="column in columns"
          :key="column.name"
          :class="[
            // recordIdx !== records.length - 1 ? 'border-b border-gray-200' : '',
            'whitespace-pre-wrap text-sm font-medium',
          ]"
        >
          <SpanTextDisplay v-if="row.text != undefined" :model-value="row" :spec="props.fields" />
          <span v-else class="font-normal">{{ row[column.name] }}</span>
        </td>
      </tr>
    </tbody>
  </table>
</template>
<script lang="ts" setup>
import type { FieldSpec } from "@/types";
import SpanTextDisplay from "@/displays/SpanTextDisplay.vue";
import { computed } from "vue";
const props = defineProps<{
  fields: Array<FieldSpec>;
  records: Array<Record<string, any>>;
}>();

// TODO @Cleanup @Architecture: support general multi-field displays & interfaces
//  (across interfaces/displays for preview, grid, form, etc.)
const columns = computed(() => props.fields.filter((f) => !["entities", "tokens"].includes(f.name)));
const rows = computed(() => props.records);
</script>
