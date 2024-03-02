<script lang="tsx" setup>
import { NodeType } from "@/proto/wire";
import { getChildren } from "@/system/graph";
import { ref } from "vue";

const Window = <div>window</div>;

const { children: subviews } = getChildren(ref(null), NodeType.VIEW);
</script>
<template>
  <div>
    <Window />
    <Window />
    <Window />
  </div>
</template>
