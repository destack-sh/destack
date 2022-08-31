<template>
  <div>{{ spec }}</div>
</template>
<script lang="ts" setup>
import type { RecordSpec, RecordType } from "@/types";

const props = defineProps<{ spec: RecordSpec | RecordType }>();
</script>
