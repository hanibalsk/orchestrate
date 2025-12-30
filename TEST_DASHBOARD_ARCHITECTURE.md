# Test Dashboard Architecture

## Component Hierarchy

```
App.tsx
└── Tests.tsx (Main Dashboard Page)
    ├── GenerateTestDialog (Action Button)
    ├── CoverageOverview (Top Left Widget)
    ├── CoverageTrend (Top Right Widget)
    ├── ModuleCoverageTable (Middle Section)
    ├── UntestedCodeList (Bottom Left)
    └── TestRunHistory (Bottom Right)
```

## Data Flow

```
API Layer (tests.ts)
    ↓
React Query (useQuery hooks)
    ↓
Tests.tsx (orchestrator)
    ↓
Individual Components (presentation)
```

## API Integration

```
Frontend                          Backend
--------                          -------
getCoverageReport()      →       GET /api/tests/coverage
getCoverageHistory()     →       GET /api/tests/coverage/history
generateTests()          →       POST /api/tests/generate
triggerTestRun()         →       POST /api/tests/run
getTestRun()             →       GET /api/tests/runs/:id
getTestSuggestions()     →       GET /api/tests/suggestions
```

## Component Details

### CoverageOverview.tsx
- Purpose: Display overall coverage summary
- Data: CoverageReport
- Features: Color-coded percentage, top modules list
- Refresh: 60s interval

### CoverageTrend.tsx
- Purpose: Show coverage over time
- Data: CoverageHistoryEntry[]
- Features: Bar chart, min/max/current stats
- Refresh: 60s interval

### ModuleCoverageTable.tsx
- Purpose: Detailed module/file breakdown
- Data: ModuleCoverage[]
- Features: Collapsible rows, progress bars
- State: Local (expanded modules)

### UntestedCodeList.tsx
- Purpose: Highlight low-coverage files
- Data: ModuleCoverage[] (filtered)
- Features: Severity badges, sorting
- Threshold: Configurable (default 50%)

### TestRunHistory.tsx
- Purpose: Recent test execution log
- Data: TestRun[]
- Features: Status icons, result counts
- Refresh: 10s interval

### GenerateTestDialog.tsx
- Purpose: Trigger test generation
- Data: None (action only)
- Features: Form validation, API mutation
- State: Modal open/close, form inputs

## Type Safety

All components use TypeScript interfaces from `test-types.ts`:
- CoverageReport
- CoverageHistoryEntry
- ModuleCoverage
- FileCoverage
- TestRun
- TestRunResults
- TestSuggestion

## Styling

- Tailwind CSS for utility classes
- Radix UI for accessible components
- Custom color scheme:
  - Success: Green (≥80%)
  - Warning: Yellow (50-79%)
  - Destructive: Red (<50%)

## Accessibility

- Semantic HTML elements
- ARIA labels on interactive elements
- Keyboard navigation support
- Screen reader compatible
- Color contrast compliance

## Performance

- React Query caching
- Auto-refresh intervals
- Lazy rendering of collapsed content
- Optimistic updates for mutations
