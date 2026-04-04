#!/bin/bash

# Bodhi Line Counter
# Counts lines of code in the project

echo "=========================================="
echo "  Bodhi Project - Line Counter"
echo "=========================================="
echo ""

TOTAL=0

# Count lines in each crate
for crate in crates/*/src; do
    if [ -d "$crate" ]; then
        crate_name=$(basename $(dirname "$crate"))
        lines=$(find "$crate" -name "*.rs" 2>/dev/null | xargs wc -l 2>/dev/null | tail -1 | awk '{print $1}')
        if [ -n "$lines" ]; then
            echo "$crate_name: $lines lines"
            TOTAL=$((TOTAL + lines))
        fi
    fi
done

echo "-------------------------------------------"
echo "  Total Rust Code: $TOTAL lines"
echo "=========================================="
echo ""

# Count by file type
echo "Breakdown by file type:"
echo "  .rs files: $(find crates -name "*.rs" 2>/dev/null | wc -l) files"
echo "  .toml files: $(find crates -name "Cargo.toml" 2>/dev/null | wc -l) files"
echo "  .md files: $(find . -maxdepth 2 -name "*.md" 2>/dev/null | wc -l) files"
