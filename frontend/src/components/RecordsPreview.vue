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
      <tr v-for="record in records" :key="record.name">
        <td
          v-for="column in columns"
          :key="column.name"
          :class="[
            // recordIdx !== records.length - 1 ? 'border-b border-gray-200' : '',
            'whitespace-pre-wrap py-2 text-sm font-medium',
          ]"
        >
          {{ record[column.name] }}
        </td>
      </tr>
    </tbody>
  </table>
</template>
<script lang="ts" setup>
import type { FieldSpec } from "@/types";
import { computed } from "vue";
const props = defineProps<{
  fields: Array<FieldSpec>;
  records: Array<Record<string, any>>;
}>();

const columns = computed(() => props.fields);
</script>
