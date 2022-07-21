<template>
  <div>{{ spec.type }}</div>
</template>
<script lang="ts" setup>
import type { RecordSpec } from "@/types";

const props = defineProps<{ spec: RecordSpec }>();
</script>
