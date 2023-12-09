<script lang="ts" setup>
import type { PageInfo } from "@/gql/graphql";

const props = defineProps<{ pageInfo: PageInfo }>();
</script>
<template>
  <div>
    <!-- nocheckin: pagination -->
    pagination
  </div>
</template>
