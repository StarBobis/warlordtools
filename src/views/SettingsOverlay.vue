<template>
  <div class="settings-overlay">
    <h2>设置</h2>
    <el-scrollbar height="100%">
      <div class="setting-item">
        <span class="label">通用设置</span>
        <el-switch v-model="value1" />
      </div>
      <el-divider />
      <div class="setting-item">
        <span class="label">透明度</span>
        <el-slider v-model="value2" />
      </div>
       <el-divider />
      <p style="font-size: 12px; color: #999;">更多设置敬请期待...</p>
    </el-scrollbar>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'

const value1 = ref(true)
const value2 = ref(50)
</script>

<style scoped>
.settings-overlay {
  position: absolute;
  top: 40px; /* Below titlebar */
  right: 0;
  width: 300px;
  bottom: 0;
  
  /* Element Dark Style matches */
  background-color: rgba(30, 30, 30, 0.95);
  backdrop-filter: blur(20px);
  border-left: 1px solid rgba(255, 255, 255, 0.1);
  
  padding: 20px;
  color: #fff;
  z-index: 100;
  box-shadow: -5px 0 15px rgba(0,0,0,0.5);
  display: flex;
  flex-direction: column;
}

.setting-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 10px;
}
.label {
    font-size: 14px;
    color: #cfd3dc;
}
</style>
