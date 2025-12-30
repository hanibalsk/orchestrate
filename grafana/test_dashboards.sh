#!/bin/bash
# Test script to validate Grafana dashboard JSON files

set -e

DASHBOARD_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/dashboards" && pwd)"
PROVISIONING_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/provisioning" && pwd)"

echo "Validating Grafana dashboard JSON files..."

# Function to validate JSON
validate_json() {
    local file=$1
    echo "  Checking: $(basename "$file")"

    # Check if file exists
    if [ ! -f "$file" ]; then
        echo "    ERROR: File does not exist"
        return 1
    fi

    # Validate JSON syntax with python
    if ! python3 -m json.tool "$file" > /dev/null 2>&1; then
        echo "    ERROR: Invalid JSON syntax"
        return 1
    fi

    # Check for required Grafana dashboard fields
    if ! grep -q '"__inputs"' "$file" 2>/dev/null && ! grep -q '"uid"' "$file" 2>/dev/null; then
        echo "    WARNING: Missing Grafana dashboard structure (uid or __inputs)"
    fi

    if ! grep -q '"panels"' "$file"; then
        echo "    ERROR: Missing 'panels' field"
        return 1
    fi

    if ! grep -q '"title"' "$file"; then
        echo "    ERROR: Missing 'title' field"
        return 1
    fi

    echo "    OK: Valid JSON with dashboard structure"
    return 0
}

# Validate all dashboard files
FAILED=0
for dashboard in "$DASHBOARD_DIR"/*.json; do
    if [ -f "$dashboard" ]; then
        if ! validate_json "$dashboard"; then
            FAILED=1
        fi
    fi
done

# Validate provisioning YAML
if [ -f "$PROVISIONING_DIR/dashboards.yaml" ]; then
    echo "  Checking: provisioning/dashboards.yaml"
    # Basic YAML validation (check if it can be read)
    if command -v yq &> /dev/null; then
        if yq eval "$PROVISIONING_DIR/dashboards.yaml" > /dev/null 2>&1; then
            echo "    OK: Valid YAML"
        else
            echo "    ERROR: Invalid YAML syntax"
            FAILED=1
        fi
    elif python3 -c "import yaml" 2>/dev/null; then
        if python3 -c "import yaml; yaml.safe_load(open('$PROVISIONING_DIR/dashboards.yaml'))" 2>/dev/null; then
            echo "    OK: Valid YAML"
        else
            echo "    ERROR: Invalid YAML syntax"
            FAILED=1
        fi
    else
        echo "    WARNING: No YAML validator found (yq or python yaml), skipping validation"
    fi
else
    echo "  WARNING: provisioning/dashboards.yaml not found"
fi

if [ $FAILED -eq 1 ]; then
    echo "FAILED: Some dashboards have validation errors"
    exit 1
else
    echo "SUCCESS: All dashboards validated"
    exit 0
fi
