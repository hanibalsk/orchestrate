#!/bin/bash

# Test script for Schedule API endpoints
BASE_URL="http://localhost:8080/api"

echo "Testing Schedule API endpoints..."
echo ""

echo "1. GET /api/schedules - List all schedules"
curl -s "$BASE_URL/schedules" | jq '.[0:2]'
echo ""

echo "2. GET /api/schedules/6 - Get specific schedule"
curl -s "$BASE_URL/schedules/6" | jq '.'
echo ""

echo "3. POST /api/schedules - Create new schedule"
curl -s -X POST "$BASE_URL/schedules" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "test-schedule",
    "cron_expression": "*/5 * * * *",
    "agent_type": "explorer",
    "task": "Test every 5 minutes",
    "enabled": true
  }' | jq '.'
echo ""

echo "4. PUT /api/schedules/9 - Update schedule"
curl -s -X PUT "$BASE_URL/schedules/9" \
  -H "Content-Type: application/json" \
  -d '{
    "enabled": false
  }' | jq '.'
echo ""

echo "5. POST /api/schedules/6/pause - Pause schedule"
curl -s -X POST "$BASE_URL/schedules/6/pause" | jq '.'
echo ""

echo "6. POST /api/schedules/6/resume - Resume schedule"
curl -s -X POST "$BASE_URL/schedules/6/resume" | jq '.'
echo ""

echo "7. GET /api/schedules/6/runs - Get execution history"
curl -s "$BASE_URL/schedules/6/runs" | jq '.'
echo ""

echo "8. POST /api/schedules/7/run - Run schedule immediately"
curl -s -X POST "$BASE_URL/schedules/7/run" | jq '.'
echo ""

echo "All tests completed!"
