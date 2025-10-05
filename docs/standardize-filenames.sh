#!/bin/bash

# Script to standardize LogLine documentation file names
# Run this script from the docs directory

echo "LogLine Documentation File Standardizer"
echo "======================================"

# Function to convert a filename to kebab-case
convert_to_kebab() {
    local filename=$1
    # Replace underscores with hyphens
    filename=$(echo "$filename" | tr '_' '-')
    # Convert to lowercase
    filename=$(echo "$filename" | tr '[:upper:]' '[:lower:]')
    # Replace spaces with hyphens
    filename=$(echo "$filename" | tr ' ' '-')
    echo $filename
}

# Process files in the docs directory
process_files() {
    for file in *.md; do
        # Skip already processed files and special files
        if [[ "$file" == "index.md" || "$file" == "naming-conventions.md" || 
              "$file" == "architecture.md" || "$file" == "vision.md" || 
              "$file" == "modularization-plan.md" || "$file" == "railway-deployment.md" ||
              "$file" == "id-refactoring-plan.md" || "$file" == "logline-id-service.md" ||
              "$file" == "logline-id-README.md" || "$file" == "logline-timeline-README.md" ]]; then
            continue
        fi
        
        # Convert file name to kebab-case
        new_name=$(convert_to_kebab "$file")
        
        # If the filename changed, rename it
        if [[ "$file" != "$new_name" ]]; then
            echo "Renaming $file to $new_name"
            mv "$file" "$new_name"
        fi
    done
}

# Main execution
echo "Starting file name standardization..."
process_files
echo "Finished standardizing file names."

# Final report
echo ""
echo "Documentation Structure:"
find . -name "*.md" | sort