<script setup lang="ts">
import { computed, ref } from "vue";
import { useOverlayWindow } from "../utils/useOverlayWindow";

const props = defineProps<{
  isActive: boolean
}>();

const containerRef = ref<HTMLElement | null>(null);

useOverlayWindow({
  label: 'poedb-overlay',
  url: 'https://poe2db.tw/cn/',
  containerRef,
  isActive: computed(() => props.isActive)
});
</script>

<template>
  <div class="poedb-page glass-panel" ref="containerRef">
     <div class="placeholder">
        <p>POEDB 窗口已嵌入显示区域</p>
        <p class="sub">若位置偏移，请拖动主窗口校准</p>
     </div>
  </div>
</template>

<style scoped>
.poedb-page {
  height: 100%;
  width: 100%;
  background: rgba(0,0,0,0.5);
  display: flex;
  align-items: center;
  justify-content: center;
}

.placeholder {
    text-align: center;
    color: #666;
    pointer-events: none;
}

.sub {
    font-size: 12px;
    opacity: 0.7;
}
</style>
