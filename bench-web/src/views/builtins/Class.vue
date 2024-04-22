<script lang="ts" setup>
import type { FieldData } from "@/proto/wire";
import type { PreparedGetConnection } from "@/system/connection";
import Field from "@/views/system/Field.vue";

const props = defineProps<{
  preparedConnection: PreparedGetConnection;
  isFunction: boolean;
  fields: FieldData[];
}>();
</script>
<template>
  <ul ref="classRef">
    <Field v-for="field in fields" :key="field.id" />
  </ul>
</template>
