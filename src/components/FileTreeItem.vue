<script setup lang="ts">
import { computed } from 'vue';

export interface FileNode {
    name: string;
    path: string;
    type: 'file' | 'dir';
    children: FileNode[];
    expanded: boolean;
}

const props = defineProps<{
    node: FileNode;
    selectedPath?: string;
    level?: number;
}>();

const emit = defineEmits<{
    (e: 'select', node: FileNode): void;
    (e: 'toggle', node: FileNode): void;
    (e: 'context-menu', event: MouseEvent, node: FileNode): void;
}>();

const isSelected = computed(() => props.node.type === 'file' && props.selectedPath === props.node.path);
const indent = computed(() => ({ paddingLeft: `${(props.level || 0) * 12 + 8}px` }));

const handleClick = () => {
    if (props.node.type === 'dir') {
        emit('toggle', props.node);
    } else {
        emit('select', props.node);
    }
};
</script>

<template>
    <div class="tree-node">
        <div 
            class="node-content" 
            :class="{ active: isSelected, 'is-folder': node.type === 'dir' }"
            :style="indent"
            @click.stop="handleClick"
            @contextmenu.prevent.stop="emit('context-menu', $event, node)"
        >
            <span class="icon" v-if="node.type === 'dir'">
                {{ node.expanded ? '📂' : '📁' }}
            </span>
            <span class="icon" v-else>📄</span>
            <span class="label">{{ node.name }}</span>
        </div>
        
        <div v-if="node.type === 'dir' && node.expanded" class="children">
            <FileTreeItem 
                v-for="child in node.children" 
                :key="child.path" 
                :node="child"
                :selectedPath="selectedPath"
                :level="(level || 0) + 1"
                @select="emit('select', $event)"
                @toggle="emit('toggle', $event)"
                @context-menu="(e, n) => emit('context-menu', e, n)"
            />
        </div>
    </div>
</template>

<style scoped>
.node-content {
    display: flex;
    align-items: center;
    padding: 4px 8px;
    cursor: pointer;
    color: #ccc;
    font-size: 13px;
    user-select: none;
    border-radius: 4px;
    gap: 6px;
    transition: background 0.1s;
}
.node-content:hover {
    background: rgba(255, 255, 255, 0.1);
}
.node-content.active {
    background: rgba(64, 158, 255, 0.2);
    color: #409eff;
}
.icon {
    opacity: 0.8;
    font-size: 14px;
    min-width: 16px;
    text-align: center;
    display: inline-block;
}
.label {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}
</style>
