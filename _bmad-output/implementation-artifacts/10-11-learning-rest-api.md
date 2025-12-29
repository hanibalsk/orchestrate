# Story 10.11: Learning REST API

Status: ready-for-dev

## Story

As a **developer**,
I want **REST API endpoints for all learning operations**,
so that **I can integrate learning features into web UI and other systems**.

## Acceptance Criteria

1. **AC1**: `POST /api/feedback` - Submit feedback on agent output
2. **AC2**: `GET /api/feedback/stats` - Get feedback statistics
3. **AC3**: `GET /api/learning/effectiveness` - Get effectiveness scores
4. **AC4**: `GET /api/learning/suggestions` - Get improvement suggestions
5. **AC5**: `POST /api/learning/analyze` - Trigger learning analysis
6. **AC6**: CRUD endpoints for experiments
7. **AC7**: `GET /api/predictions` - Get task predictions

## Tasks / Subtasks

- [ ] Task 1: Feedback endpoints (AC: 1, 2)
  - [ ] 1.1: POST /api/feedback - create feedback
  - [ ] 1.2: GET /api/feedback - list feedback with filters
  - [ ] 1.3: GET /api/feedback/stats - aggregate statistics

- [ ] Task 2: Learning endpoints (AC: 3, 4, 5)
  - [ ] 2.1: GET /api/learning/effectiveness - effectiveness table
  - [ ] 2.2: GET /api/learning/suggestions - improvement suggestions
  - [ ] 2.3: POST /api/learning/analyze - trigger analysis
  - [ ] 2.4: GET /api/learning/patterns - list observed patterns

- [ ] Task 3: Experiment endpoints (AC: 6)
  - [ ] 3.1: GET /api/experiments - list experiments
  - [ ] 3.2: POST /api/experiments - create experiment
  - [ ] 3.3: GET /api/experiments/:id - get experiment details
  - [ ] 3.4: GET /api/experiments/:id/results - get results
  - [ ] 3.5: POST /api/experiments/:id/promote - promote variant

- [ ] Task 4: Prediction endpoint (AC: 7)
  - [ ] 4.1: POST /api/predictions - predict task outcomes
  - [ ] 4.2: Return structured prediction response

- [ ] Task 5: Write tests
  - [ ] 5.1: Integration tests for each endpoint
  - [ ] 5.2: Tests for error handling
  - [ ] 5.3: Tests for authentication

## Dev Notes

### API Response Formats

```json
// POST /api/feedback
{
  "agent_id": "uuid",
  "message_id": 123,
  "rating": "positive",
  "comment": "Great response!"
}

// GET /api/learning/effectiveness
{
  "instructions": [
    {
      "id": 1,
      "name": "learned_error_01",
      "effectiveness": 0.852,
      "usage_count": 234,
      "enabled": true
    }
  ]
}

// POST /api/predictions
// Request:
{ "task": "Implement OAuth2 login flow", "agent_type": "story-developer" }
// Response:
{
  "success_probability": 0.78,
  "token_range": { "min": 45000, "max": 65000 },
  "duration_seconds": { "min": 1500, "max": 2400 },
  "recommended_model": "claude-3-opus",
  "risk_factors": ["Complex integration"],
  "recommendations": ["Break into subtasks"]
}
```

### References

- [Source: docs/bmad/epics/epic-010-closed-loop-learning.md#Story 11]
- [Source: crates/orchestrate-web/src/lib.rs] - Existing API patterns

## Dev Agent Record

### Agent Model Used
### Debug Log References
### Completion Notes List
### File List
