# Story 10.12: Learning Dashboard UI

Status: ready-for-dev

## Story

As a **user**,
I want **web dashboard pages for learning features**,
so that **I can visualize and manage learning data through a GUI**.

## Acceptance Criteria

1. **AC1**: Learning overview page with key metrics (success rate, active instructions, patterns)
2. **AC2**: Instruction effectiveness chart (bar chart or table)
3. **AC3**: Experiment management interface (create, view, promote)
4. **AC4**: Feedback submission UI (thumbs up/down on agent messages)
5. **AC5**: Pattern browser with filtering and search
6. **AC6**: Suggestion review interface with approve/reject actions
7. **AC7**: Prediction tool UI

## Tasks / Subtasks

- [ ] Task 1: Learning overview page (AC: 1)
  - [ ] 1.1: Create /learning route and template
  - [ ] 1.2: Display key metrics: success rate trend, instruction count, pattern count
  - [ ] 1.3: Show recent activity feed

- [ ] Task 2: Effectiveness visualization (AC: 2)
  - [ ] 2.1: Create effectiveness chart component
  - [ ] 2.2: Sortable table of instructions with metrics
  - [ ] 2.3: Enable/disable toggles

- [ ] Task 3: Experiment UI (AC: 3)
  - [ ] 3.1: Experiment list page
  - [ ] 3.2: Experiment detail page with metrics per variant
  - [ ] 3.3: Create experiment form
  - [ ] 3.4: Promote variant button

- [ ] Task 4: Feedback UI (AC: 4)
  - [ ] 4.1: Add feedback buttons to agent message display
  - [ ] 4.2: Feedback submission modal
  - [ ] 4.3: Feedback history view

- [ ] Task 5: Pattern browser (AC: 5)
  - [ ] 5.1: Pattern list with type filters
  - [ ] 5.2: Pattern detail modal
  - [ ] 5.3: Search by pattern signature

- [ ] Task 6: Suggestion review UI (AC: 6)
  - [ ] 6.1: Pending suggestions list
  - [ ] 6.2: Approve/reject buttons
  - [ ] 6.3: Preview of generated instruction

- [ ] Task 7: Prediction UI (AC: 7)
  - [ ] 7.1: Task input form
  - [ ] 7.2: Prediction results display
  - [ ] 7.3: Risk factors and recommendations

- [ ] Task 8: Write tests
  - [ ] 8.1: Template rendering tests
  - [ ] 8.2: API integration tests

## Dev Notes

### UI Framework

The project uses:
- Askama templates for HTML
- HTMX for dynamic updates (likely)
- Axum for routing

### Page Structure

```
/learning
├── /learning              # Overview dashboard
├── /learning/instructions # Instruction effectiveness
├── /learning/patterns     # Pattern browser
├── /learning/suggestions  # Pending suggestions
├── /learning/experiments  # Experiment management
└── /learning/predictions  # Prediction tool
```

### Key Templates to Create

- `templates/learning/index.html` - Overview
- `templates/learning/instructions.html` - Effectiveness table
- `templates/learning/experiments.html` - Experiment list
- `templates/learning/experiment_detail.html` - Single experiment
- `templates/learning/patterns.html` - Pattern browser
- `templates/learning/predictions.html` - Prediction form

### References

- [Source: docs/bmad/epics/epic-010-closed-loop-learning.md#Story 12]
- [Source: crates/orchestrate-web/src/lib.rs] - Web routing
- [Source: frontend/] - Existing frontend code

## Dev Agent Record

### Agent Model Used
### Debug Log References
### Completion Notes List
### File List
