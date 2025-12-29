# Testing the Schedule Dashboard UI

## Quick Start

### 1. Start the Web Server

```bash
cd /Users/martinjanci/projects/github.com/hanibalsk/orchestrate/.worktrees/epic-003-scheduling
./target/release/orchestrate web --port 8080
```

The server will start at http://localhost:8080

### 2. Access the Schedule Dashboard

Open your browser and navigate to:
```
http://localhost:8080/schedules
```

### 3. Test Features

#### View Schedules
- The main page shows all existing schedules in a table
- Notice the real-time countdown to next run (updates every second)
- Status badges show enabled (green) or disabled (gray)
- Filter schedules using the dropdown (All/Enabled/Disabled)

#### Create New Schedule
1. Click "Create Schedule" button
2. Fill in the form:
   - **Name**: e.g., "test-schedule"
   - **Frequency**: Choose from presets or select "Custom expression"
   - **Agent Type**: Select from available agents
   - **Task**: Describe what the agent should do
   - **Enable**: Check to activate immediately
3. Click "Create Schedule"
4. The new schedule appears in the table

#### Manage Schedules
- **Pause/Resume**: Click the pause/play button to toggle schedule status
- **Run Now**: Click the play circle button to execute immediately
- **Delete**: Click the trash button to remove (shows confirmation)

#### View Execution History
1. Click on any schedule row in the table
2. A dialog opens showing execution history:
   - Start time
   - Duration (live updates for running executions)
   - Status (running/completed/failed)
   - Agent ID (clickable link to agent detail)
   - Error message (if failed)

### 4. Test with CLI

#### Create Test Data
```bash
# Create various schedules
./target/release/orchestrate schedule add \
  --name "every-5-min" \
  --cron "*/5 * * * *" \
  --agent explorer \
  --task "Check system every 5 minutes"

./target/release/orchestrate schedule add \
  --name "morning-report" \
  --cron "0 9 * * 1-5" \
  --agent code_reviewer \
  --task "Generate morning code quality report"
```

#### Trigger Execution
```bash
# Run a schedule immediately
./target/release/orchestrate schedule run-now every-5-min

# This creates a run entry that will show in the history
```

#### Check Execution History
```bash
# View history in CLI
./target/release/orchestrate schedule history every-5-min

# Then view the same data in the UI by clicking the schedule
```

### 5. Test API Directly

Use the provided test script:
```bash
chmod +x test_schedule_api.sh
./test_schedule_api.sh
```

This tests all REST API endpoints:
- GET /api/schedules
- POST /api/schedules
- PUT /api/schedules/:id
- DELETE /api/schedules/:id
- POST /api/schedules/:id/pause
- POST /api/schedules/:id/resume
- POST /api/schedules/:id/run
- GET /api/schedules/:id/runs

### 6. Verify Real-time Features

#### Countdown Updates
1. Create a schedule with `@hourly` or near-future time
2. Watch the "Next Run" column update every second
3. The countdown shows: "Xd Xh", "Xh Xm", "Xm Xs", or "Xs"

#### Auto-refresh
- Leave the page open
- Modify a schedule via CLI
- Wait up to 30 seconds
- The page automatically refreshes the data

#### Live Run Duration
1. Create a long-running schedule
2. Trigger it manually
3. Click to view history while it's running
4. Watch the duration update in real-time

### 7. Test Error Handling

#### Invalid Cron Expression
1. Click "Create Schedule"
2. Select "Custom expression"
3. Enter invalid cron: "invalid"
4. Try to submit
5. Should see validation error

#### Delete Confirmation
1. Click delete button on any schedule
2. Verify confirmation dialog appears
3. Test both cancel and confirm

#### Network Error Simulation
1. Stop the web server
2. Try to create/pause/delete a schedule
3. Verify error handling (error states shown)

### 8. Test Responsive Design

Test the UI at different screen sizes:
- Desktop (1920x1080)
- Tablet (768px)
- Mobile (375px)

The table should remain usable with horizontal scroll if needed.

### 9. Accessibility Testing

#### Keyboard Navigation
- Tab through all interactive elements
- Press Enter to activate buttons
- Test form submission with Enter key

#### Screen Reader
- All form fields have proper labels
- Buttons have descriptive text/titles
- Table headers are properly marked up

### 10. Performance Testing

#### Many Schedules
Create 20+ schedules and verify:
- Table loads quickly
- Filtering works smoothly
- Real-time updates don't cause lag
- Pagination might be needed for 100+ schedules

#### Long Histories
Create multiple runs for a schedule and verify:
- History dialog loads quickly
- Scrolling is smooth
- Data displays correctly

## Expected Behavior Summary

### Schedule List Page
- Shows all schedules with current status
- Updates countdown every second
- Auto-refreshes every 30 seconds
- Filters work correctly
- Actions succeed with optimistic updates

### Create Dialog
- Form validation works
- Preset picker updates cron expression
- Custom expressions accepted
- Submit creates schedule and updates list

### History Dialog
- Shows execution history
- Links to agent details work
- Running executions show live duration
- Error messages truncate appropriately

### Actions
- Pause/Resume toggle updates immediately
- Run Now creates agent and run record
- Delete shows confirmation and removes schedule

## Known Limitations

1. **Pagination**: Currently shows all schedules. For 100+ schedules, pagination should be added.
2. **Cron Validation**: Frontend only validates non-empty. Server validates syntax.
3. **Timezone**: All times shown in UTC. Localization could be added.
4. **Edit Schedule**: Currently only create/delete. Edit functionality could be added.

## Troubleshooting

### UI Not Loading
- Check web server is running
- Verify port 8080 is not in use
- Check browser console for errors

### Schedules Not Showing
- Verify database has schedules: `./target/release/orchestrate schedule list`
- Check API responds: `curl http://localhost:8080/api/schedules`

### Real-time Updates Not Working
- Check browser console for JavaScript errors
- Verify React Query is configured correctly
- Try hard refresh (Cmd+Shift+R / Ctrl+Shift+R)

### Countdown Not Updating
- Check browser performance settings
- Verify setInterval is not being throttled
- Try refreshing the page

## Success Criteria

All acceptance criteria should work:
- ✅ Schedule list page showing all schedules
- ✅ Visual indicator for enabled/disabled
- ✅ Next run countdown display
- ✅ Create schedule form with cron builder
- ✅ Execution history view
- ✅ Manual trigger button
- ✅ Pause/resume toggle

The UI should be:
- Fast and responsive
- Intuitive to use
- Error-tolerant
- Accessible
- Visually consistent with existing pages
