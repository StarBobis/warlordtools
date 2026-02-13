<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { configManager } from "../utils/ConfigManager";
import { FilterParser, type FilterBlock } from '../utils/FilterParser';
import FilterRuleEditor from '../components/FilterRuleEditor.vue';

interface FilterFile {
  name: string;
  path: string;
}

const filterFiles = ref<FilterFile[]>([]);
const selectedFile = ref<FilterFile | null>(null);
const parsedBlocks = ref<FilterBlock[]>([]);
const isLoading = ref(false);
const saveStatus = ref<string>("");
const currentViewMode = ref<'visual' | 'code'>('visual');
const rawContent = ref<string>("");

const scanFilters = async () => {
  const settings = configManager.getSettings();
  if (!settings.filterStoragePath) {
    alert("请先在设置中配置过滤器存储路径");
    return;
  }

  isLoading.value = true;
  try {
    const files = await invoke<string[]>("scan_filter_files", {
      path: settings.filterStoragePath,
    });
    
    filterFiles.value = files
      .map((path) => {
        const name = path.split(/[/\\]/).pop() || path;
        return { name, path };
      })
      .sort((a, b) => a.name.localeCompare(b.name));
      
  } catch (error) {
    console.error("Failed to scan filters:", error);
    alert(`扫描失败: ${error}`);
  } finally {
    isLoading.value = false;
  }
};

const selectFile = async (file: FilterFile) => {
  selectedFile.value = file;
  isLoading.value = true;
  saveStatus.value = "";
  parsedBlocks.value = [];
  try {
    const content = await invoke<string>("read_file_content", {
      path: file.path,
    });
    rawContent.value = content;
    // Parse immediately
    parsedBlocks.value = FilterParser.parse(content);
  } catch (error) {
    console.error("Failed to read file:", error);
    alert(`读取失败: ${error}`);
  } finally {
    isLoading.value = false;
  }
};

const saveFile = async () => {
  if (!selectedFile.value) return;
  
  try {
    let contentToSave = "";
    if (currentViewMode.value === 'visual') {
        // Regenerate from blocks
        contentToSave = FilterParser.stringify(parsedBlocks.value);
        // Sync back to raw for consistency if we switch tabs
        rawContent.value = contentToSave; 
    } else {
        contentToSave = rawContent.value;
        // Sync to blocks? Maybe tricky if syntax error.
        // Let's try to parse silently to update visual view if valid
        try {
            parsedBlocks.value = FilterParser.parse(contentToSave);
        } catch(e) { /* ignore */ }
    }

    await invoke("write_file_content", {
      path: selectedFile.value.path,
      content: contentToSave,
    });
    saveStatus.value = "保存成功!";
    setTimeout(() => saveStatus.value = "", 3000);
  } catch (error) {
    console.error("Failed to save file:", error);
    alert(`保存失败: ${error}`);
  }
};

const switchMode = (mode: 'visual' | 'code') => {
    if (mode === currentViewMode.value) return;
    
    if (mode === 'visual') {
        // Switching to visual: Parse current raw text
        parsedBlocks.value = FilterParser.parse(rawContent.value);
    } else {
        // Switching to code: Stringify current blocks
        rawContent.value = FilterParser.stringify(parsedBlocks.value);
    }
    currentViewMode.value = mode;
}

const addNewBlock = () => {
    parsedBlocks.value.unshift({
        id: crypto.randomUUID(),
        type: 'Show',
        name: 'New Rule',
        category: 'Custom',
        priority: 'Normal',
        rawHeader: 'Custom - New Rule - Normal',
        lines: []
    });
};

onMounted(() => {
    const settings = configManager.getSettings();
    if(settings.filterStoragePath) {
        scanFilters();
    }
});
</script>

<template>
  <div class="filter-page">
    <!-- Sidebar -->
    <div class="sidebar glass-panel">
      <div class="sidebar-header">
        <h3>过滤器列表</h3>
        <button class="glass-button small" @click="scanFilters">刷新</button>
      </div>
      <div class="file-list">
        <div 
          v-for="file in filterFiles" 
          :key="file.path"
          class="file-item"
          :class="{ active: selectedFile?.path === file.path }"
          @click="selectFile(file)"
        >
          <span class="file-icon">📄</span>
          <span class="file-name">{{ file.name }}</span>
        </div>
      </div>
    </div>

    <!-- Main Content -->
    <div class="main-content glass-panel" v-if="selectedFile">
      <!-- Toolbar -->
      <div class="editor-header">
           <div class="left-tools">
               <span class="current-file">{{ selectedFile.name }}</span>
               <div class="mode-switch">
                   <button 
                    class="glass-button small" 
                    :class="{ active: currentViewMode === 'visual' }"
                    @click="switchMode('visual')">可视化</button>
                   <button 
                    class="glass-button small" 
                    :class="{ active: currentViewMode === 'code' }"
                    @click="switchMode('code')">代码</button>
               </div>
           </div>
           
           <div class="right-tools">
               <span class="save-status">{{ saveStatus }}</span>
               <button v-if="currentViewMode === 'visual'" class="glass-button" @click="addNewBlock">+ 添加规则</button>
               <button class="glass-button primary" @click="saveFile">保存文件</button>
           </div>
      </div>

      <!-- VISUAL EDITOR -->
      <div v-show="currentViewMode === 'visual'" class="visual-editor-container">
          <div class="blocks-list">
              <FilterRuleEditor 
                v-for="(block, index) in parsedBlocks" 
                :key="block.id"
                :block="block"
                @delete="parsedBlocks.splice(index, 1)"
              />
              <div v-if="parsedBlocks.length === 0" class="empty-blocks">
                  没有找到规则，或解析失败。请尝试切换到代码模式查看。
              </div>
          </div>
      </div>

      <!-- CODE EDITOR -->
      <textarea 
          v-show="currentViewMode === 'code'"
          v-model="rawContent" 
          class="code-editor" 
          spellcheck="false"
      ></textarea>
    </div>

    <div v-else class="welcome-state glass-panel main-content">
        <p>请选择左侧文件</p>
    </div>
  </div>
</template>

<style scoped>
.filter-page {
  display: flex;
  height: 100%;
  width: 100%;
  gap: 16px;
  padding: 16px;
  box-sizing: border-box;
  color: #eee;
}

.glass-panel {
  background: rgba(30, 30, 30, 0.6);
  backdrop-filter: blur(10px);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 8px;
  display: flex;
  flex-direction: column;
}

.sidebar { width: 220px; flex-shrink: 0; }
.main-content { flex: 1; overflow: hidden; }

.sidebar-header, .editor-header {
  padding: 10px 16px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.left-tools, .right-tools {
    display: flex;
    align-items: center;
    gap: 12px;
}

.file-list { flex: 1; overflow-y: auto; padding: 8px; }

.file-item {
  padding: 8px;
  border-radius: 4px;
  cursor: pointer;
  color: #ccc;
  display: flex; align-items: center; gap: 8px;
  font-size: 13px;
}
.file-item:hover { background: rgba(255,255,255,0.1); }
.file-item.active { background: rgba(64,158,255,0.2); color: #409eff; }

.visual-editor-container {
    flex: 1;
    overflow-y: auto;
    padding: 12px;
}

.code-editor {
    flex: 1;
    background: rgba(0,0,0,0.3);
    color: #d4d4d4;
    border: none;
    padding: 16px;
    font-family: monospace;
    resize: none;
    outline: none;
}

.mode-switch {
    display: flex;
    background: rgba(0,0,0,0.2);
    border-radius: 4px;
    padding: 2px;
}

.mode-switch button {
    border: none;
    background: transparent;
    opacity: 0.6;
}

.mode-switch button.active {
    background: rgba(255,255,255,0.1);
    opacity: 1;
}

/* Reusing glass button styles */
.glass-button {
  background: rgba(255, 255, 255, 0.1);
  border: 1px solid rgba(255, 255, 255, 0.2);
  color: #eee;
  padding: 5px 10px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 12px;
}
.glass-button:hover { background: rgba(255, 255, 255, 0.2); }
.glass-button.primary { background: rgba(64, 158, 255, 0.4); border-color: rgba(64, 158, 255, 0.6); }

.save-status { color: #67c23a; font-size: 12px; }
.welcome-state { align-items: center; justify-content: center; color: #666; }
</style>
