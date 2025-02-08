<script lang="ts" setup>
import { BlockData, PageData } from "@/proto/wire";
import { PreparedGetConnection } from "@/system/connection";

const props = defineProps<{
  page: PageData;
  blocks: BlockData[];
  beforeBlock: BlockData | undefined;
  afterBlock: BlockData | undefined;
  connection: PreparedGetConnection;
}>();
const { graph, connection } = props.connection;
</script>
<template>
  <div>
    <!-- nocheckin -->
    <!-- TextBlockGroup -->
    TextBlockGroup count:{{ blocks.length }} before: {{ beforeBlock?.id }} after: {{ afterBlock?.id }}
  </div>
</template>
