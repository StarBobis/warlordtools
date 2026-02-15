export interface FilterLine {
    key: string;
    operator?: string;
    values: string[];
    raw: string; // Preserve original formatting just in case
}

export interface FilterBlock {
    id: string;
    type: string; // Show, Hide, Minimal, Continue
    
    // Position info
    startLine: number;

    // Structured Header Fields
    category?: string; // e.g. "基础"
    name?: string;     // e.g. "货币通货"
    priority?: string; // e.g. "优先级1"
    
    // Fallback for non-standard headers
    rawHeader: string; // User-entered comment text (no leading #), may span multiple lines

    // Inline comments inside the block body, paired with positions for stable round-trip
    inlineComments: Array<{ before: number; text: string }>;

    lines: FilterLine[]; 
}

export class FilterParser {
    static parse(content: string): FilterBlock[] {
        const blocks: FilterBlock[] = [];
        const lines = content.split(/\r?\n/);
        
        let currentHeaderLines: string[] = [];
        let headerStartLine = -1;
        let currentBlock: FilterBlock | null = null;
        
        const blockStartKeywords = ['Show', 'Hide', 'Minimal', 'Continue'];

        for (let i = 0; i < lines.length; i++) {
            const rawLine = lines[i];
            const trimmedLine = rawLine.trim();
            const isIndented = rawLine.startsWith(' ') || rawLine.startsWith('\t');
            
            // Empty lines often separate blocks, but can exist within a block (rarely in strict filters but possible)
            // In this format, empty lines separate blocks.
            if (!trimmedLine) {
                continue;
            }

            // Comment Processing
            if (trimmedLine.startsWith('#')) {
                // Header comments (before a block starts)
                if (!currentBlock || !isIndented) {
                    if (headerStartLine === -1) {
                        headerStartLine = i;
                    }
                    // Keep raw text without leading '#'
                    currentHeaderLines.push(trimmedLine.replace(/^#\s?/, ''));
                    continue;
                }

                // Inline comment inside a block; preserve with position information
                currentBlock.inlineComments.push({ before: currentBlock.lines.length, text: rawLine.trimEnd() });
                continue;
            }

            // Block Start Detection
            const firstToken = trimmedLine.split(/\s+/)[0];
            if (blockStartKeywords.includes(firstToken) && !isIndented) {
                
                // Flush previous block
                if (currentBlock) {
                    blocks.push(currentBlock);
                }

                // Parse Header
                // Rule: Take the last non-empty comment line as the definition
                // Format: "Category - Name - Priority"
                const headerText = currentHeaderLines.length > 0 ? currentHeaderLines[currentHeaderLines.length - 1] : "";
                
                let category = "";
                let name = "";
                let priority = "";

                // Strict splitting by " - " based on user examples
                const parts = headerText.split(' - ').map(s => s.trim());
                
                if (parts.length >= 3) {
                    // Full format: "基础 - 货币通货 - 优先级1"
                    category = parts[0];
                    name = parts[1];
                    priority = parts[2];
                } else if (parts.length === 2) {
                    // Partial: "Category - Name" (Assumption)
                    category = parts[0];
                    name = parts[1];
                } else {
                    // Fallback using raw text if no separators found
                    name = headerText;
                }

                // Mode/Pattern handling (e.g. "# 模式/Mode: 物品优先/Traditional")
                // If it doesn't look like a standard rule header, we might just store it in rawHeader.
                // But the user wants to "fix design errors", likely wanting consistent parsing.
                
                currentBlock = {
                    id: crypto.randomUUID(),
                    type: firstToken,
                    startLine: headerStartLine !== -1 ? headerStartLine : i,
                    category,
                    name,
                    priority,
                    rawHeader: currentHeaderLines.join('\n'), // Store full header just in case
                    inlineComments: [],
                    lines: []
                };
                
                currentHeaderLines = [];
                headerStartLine = -1;
                continue;
            }

            // Line Processing
            if (currentBlock) {
                const parsedLine = this.parseLine(trimmedLine);
                if (parsedLine) {
                     // Auto-Merge logic for list-type keys (BaseType, Class)
                     // Because "Liquid Despair", "Liquid Disgust" share the same attributes
                     const mergeableKeys = ['BaseType', 'Class', 'Prophecy'];
                     
                     const existingLine = currentBlock.lines.find(l => l.key === parsedLine.key);
                     
                     if (existingLine && mergeableKeys.includes(parsedLine.key)) {
                         existingLine.values.push(...parsedLine.values);
                         // Append raw for fidelity (optional, but good for debug)
                         existingLine.raw += " " + parsedLine.values.join(' ');
                     } else {
                         currentBlock.lines.push(parsedLine);
                     }
                }
            }
        }

        // Flush last block
        if (currentBlock) {
            blocks.push(currentBlock);
        }

        return blocks;
    }

    static stringify(blocks: FilterBlock[]): string {
        let output = "";
        let currentLine = 0;

        for (const block of blocks) {
            // Update the block's start position to match the generated matches
            block.startLine = currentLine;

            // Reconstruct Header: always honor rawHeader (textarea) first
            const headerLines: string[] = [];

            if (block.rawHeader && block.rawHeader.trim()) {
                headerLines.push(...block.rawHeader.split('\n').map(l => l.trimEnd()).filter(l => l.length > 0));
            } else if (block.name) {
                let headerLine = "";
                if (block.category) {
                    headerLine = `${block.category} - ${block.name}`;
                    if (block.priority) headerLine += ` - ${block.priority}`;
                } else {
                    headerLine = block.name;
                }
                headerLines.push(headerLine);
            }

            headerLines.forEach((h) => {
                const line = h.startsWith('#') ? h : `# ${h}`;
                output += `${line}\n`;
                currentLine += 1;
            });
            
            // Block Type
            output += `${block.type}\n`;
            currentLine += 1;

            const inlineComments = block.inlineComments ?? [];

            // Lines
            for (let idx = 0; idx < block.lines.length; idx++) {
                // Emit inline comments that appear before this line index
                const beforeComments = inlineComments.filter(c => c.before === idx);
                beforeComments.forEach(c => {
                    const text = c.text.trim().startsWith('#') ? c.text.trimEnd() : `# ${c.text.trimEnd()}`;
                    output += `    ${text}\n`;
                    currentLine += 1;
                });

                const line = block.lines[idx];
                let lineStr = `    ${line.key}`;
                if (line.operator) {
                    lineStr += ` ${line.operator}`;
                }
                
                if (line.values.length > 0) {
                     const valStr = line.values.join(' ');
                     lineStr += ` ${valStr}`;
                }

                output += `${lineStr}\n`;
                currentLine += 1;
            }

            // Trailing inline comments (those targeting end of block)
            const trailingComments = inlineComments.filter(c => c.before >= block.lines.length);
            trailingComments.forEach(c => {
                const text = c.text.trim().startsWith('#') ? c.text.trimEnd() : `# ${c.text.trimEnd()}`;
                output += `    ${text}\n`;
                currentLine += 1;
            });
            
            output += "\n";
            currentLine += 1;
        }

        return output;
    }

    private static parseLine(line: string): FilterLine | null {
        const trimmed = line.trim();
        if (!trimmed || trimmed.startsWith('#')) return null;

        // Naive split by space acts weird with quotes.
        // We need a regex or state machine for quotes.
        
        const parts: string[] = [];
        let buffer = "";
        let inQuote = false;
        
        for (let i = 0; i < trimmed.length; i++) {
            const char = trimmed[i];
            
            if (char === '"') {
                inQuote = !inQuote;
                buffer += char;
            } else if (char === ' ' && !inQuote) {
                if (buffer.length > 0) {
                    parts.push(buffer);
                    buffer = "";
                }
            } else {
                buffer += char;
            }
        }
        // Push last buffer
        if (buffer.length > 0) {
            parts.push(buffer);
        }

        if (parts.length === 0) return null;

        const key = parts[0];
        let operator: string | undefined = undefined;
        let valueStartIndex = 1;

        // Check for operator in second position
        // Common operators in PoE Filters
        const operators = ['==', '=', '<', '>', '<=', '>='];
        if (parts.length > 1 && operators.includes(parts[1])) {
            operator = parts[1];
            valueStartIndex = 2;
        }

        let values = parts.slice(valueStartIndex);

        // Sanitize values (remove trailing commas that might have stuck due to splitting logic)
        // User reports "BaseType A, B" style inputs
        values = values.map(v => v.replace(/,$/, '').replace(/^,/, ''));
        // Remove empty values that might result from ", " -> split -> "," -> replace -> ""
        values = values.filter(v => v.length > 0);

        return {
            key,
            operator,
            values,
            raw: line
        };
    }
}