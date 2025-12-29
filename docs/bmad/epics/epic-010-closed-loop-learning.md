# Epic 010: Closed-Loop Learning

Implement comprehensive learning from all outcomes to enable continuous self-improvement.

**Priority:** High
**Effort:** Large
**Use Cases:** UC-501, UC-505

## Overview

Currently, orchestrate only learns from failures. This epic expands the learning system to learn from successes, user feedback, and performance metrics. The system will automatically optimize prompts, model selection, and agent behavior based on observed outcomes.

## Stories

### Story 1: Success Pattern Detection

Learn from successful agent completions.

**Acceptance Criteria:**
- [ ] Analyze successful task completions
- [ ] Identify common patterns in successful prompts
- [ ] Identify effective tool usage sequences
- [ ] Track context that leads to success
- [ ] Store success patterns in database
- [ ] `orchestrate learn successes` command

**Success Factors to Track:**
- Prompt structure that works
- Tool call sequences that are efficient
- Context size that's optimal
- Model choice for task type
- Time of day (API performance)

### Story 2: User Feedback Collection

Collect explicit feedback on agent outputs.

**Acceptance Criteria:**
- [ ] Add feedback endpoints (thumbs up/down)
- [ ] Feedback buttons in web UI
- [ ] Feedback via Slack reactions
- [ ] Store feedback with agent context
- [ ] Link feedback to specific outputs
- [ ] `orchestrate feedback add --agent <id> --rating <pos|neg> --comment <text>`

**Feedback Model:**
```rust
struct Feedback {
    id: i64,
    agent_id: Uuid,
    message_id: Option<i64>,
    rating: FeedbackRating,  // Positive, Negative, Neutral
    comment: Option<String>,
    created_at: DateTime<Utc>,
    created_by: String,
}
```

### Story 3: Effectiveness Scoring

Calculate effectiveness scores for instructions and patterns.

**Acceptance Criteria:**
- [ ] Track instruction usage across agents
- [ ] Correlate instructions with outcomes
- [ ] Calculate effectiveness = successes / (successes + failures)
- [ ] Weight by recency (recent outcomes matter more)
- [ ] Identify ineffective instructions automatically
- [ ] Suggest instruction improvements

**Scoring Algorithm:**
```
effectiveness = (
    0.7 * recent_success_rate +  // Last 7 days
    0.2 * historical_success_rate +  // All time
    0.1 * user_feedback_score
)
```

### Story 4: A/B Testing Framework

Test prompt variations systematically.

**Acceptance Criteria:**
- [ ] Create experiments with variant prompts
- [ ] Randomly assign agents to variants
- [ ] Track metrics per variant
- [ ] Statistical significance calculation
- [ ] Auto-promote winning variant
- [ ] `orchestrate experiment create --name <name> --variants <file>`

**Experiment Definition:**
```yaml
experiment:
  name: code-review-prompt-v2
  hypothesis: "More specific review criteria improves fix rate"
  metric: review_fix_rate
  variants:
    - name: control
      prompt: prompts/code-reviewer.md
      weight: 50
    - name: treatment
      prompt: prompts/code-reviewer-v2.md
      weight: 50
  min_samples: 100
  confidence_level: 0.95
```

### Story 5: Dynamic Model Selection

Choose optimal model based on task characteristics.

**Acceptance Criteria:**
- [ ] Classify tasks by complexity
- [ ] Track success rate by model per task type
- [ ] Track cost by model per task type
- [ ] Recommend optimal model for new tasks
- [ ] Support cost/quality tradeoff preference
- [ ] Auto-select model based on learned patterns

**Model Selection Logic:**
```yaml
model_selection:
  rules:
    - task_type: simple_fix
      preferred: claude-3-haiku
      fallback: claude-3-sonnet

    - task_type: complex_refactor
      preferred: claude-3-opus

    - task_type: code_review
      preferred: claude-3-sonnet

  optimization:
    goal: balanced  # cost, quality, balanced
    max_cost_per_task: 0.50
```

### Story 6: Prompt Optimization

Automatically improve prompts based on outcomes.

**Acceptance Criteria:**
- [ ] Track prompt effectiveness metrics
- [ ] Identify prompt sections that correlate with failures
- [ ] Suggest prompt improvements
- [ ] Test improvements via A/B testing
- [ ] Version prompts with effectiveness history
- [ ] `orchestrate prompt optimize --agent-type <type>`

**Optimization Areas:**
- Instruction clarity
- Example quality
- Constraint effectiveness
- Context relevance
- Output format specifications

### Story 7: Cross-Project Pattern Learning

Share effective patterns across projects.

**Acceptance Criteria:**
- [ ] Export learned instructions to portable format
- [ ] Import instructions from other projects
- [ ] Pattern matching for similar projects
- [ ] Confidence adjustment for imported patterns
- [ ] Pattern marketplace (optional)
- [ ] `orchestrate learn export --output patterns.yaml`
- [ ] `orchestrate learn import --file patterns.yaml`

**Pattern Export Format:**
```yaml
patterns:
  - id: pat-001
    type: instruction
    content: "Always check for null before accessing properties"
    context:
      language: typescript
      agent_types: [story-developer, issue-fixer]
    effectiveness:
      success_rate: 0.87
      sample_size: 234

  - id: pat-002
    type: tool_sequence
    content: ["read_file", "grep", "edit_file", "run_tests"]
    context:
      task_type: bug_fix
    effectiveness:
      success_rate: 0.91
      sample_size: 156
```

### Story 8: Feedback Loop Automation

Automate the improvement cycle.

**Acceptance Criteria:**
- [ ] Scheduled learning analysis (daily)
- [ ] Auto-generate instruction suggestions
- [ ] Auto-disable ineffective instructions
- [ ] Auto-promote successful experiments
- [ ] Learning summary reports
- [ ] `orchestrate learn auto --enable` command

**Daily Learning Cycle:**
1. Analyze previous day's outcomes
2. Update effectiveness scores
3. Identify new patterns
4. Suggest new instructions
5. Review experiment results
6. Generate learning report

### Story 9: Performance Prediction

Predict task outcomes before execution.

**Acceptance Criteria:**
- [ ] Build prediction model from historical data
- [ ] Predict success probability for new tasks
- [ ] Predict token usage
- [ ] Predict completion time
- [ ] Warn on low-probability tasks
- [ ] `orchestrate predict --task <description>` command

**Prediction Output:**
```
Task Prediction

Description: "Implement OAuth2 login flow"

Predictions:
  Success probability: 78%
  Estimated tokens: 45,000 - 65,000
  Estimated duration: 25-40 minutes
  Recommended model: claude-3-opus

Risk factors:
  - Similar tasks had 22% failure rate
  - Complex integration required

Recommendations:
  - Break into smaller subtasks
  - Ensure OAuth library docs are available
```

### Story 10: Learning CLI Commands

Comprehensive CLI for learning operations.

**Acceptance Criteria:**
- [ ] `orchestrate learn analyze` - Run learning analysis
- [ ] `orchestrate learn successes` - Analyze successful patterns
- [ ] `orchestrate learn effectiveness` - Show instruction effectiveness
- [ ] `orchestrate learn suggest` - Get improvement suggestions
- [ ] `orchestrate learn export` - Export patterns
- [ ] `orchestrate learn import` - Import patterns
- [ ] `orchestrate learn auto --enable/--disable` - Toggle automation
- [ ] `orchestrate experiment create/list/results/promote`
- [ ] `orchestrate feedback add/list/stats`

### Story 11: Learning REST API

Add REST endpoints for learning.

**Acceptance Criteria:**
- [ ] `POST /api/feedback` - Submit feedback
- [ ] `GET /api/feedback/stats` - Feedback statistics
- [ ] `GET /api/learning/effectiveness` - Effectiveness scores
- [ ] `GET /api/learning/suggestions` - Improvement suggestions
- [ ] `POST /api/learning/analyze` - Trigger analysis
- [ ] `GET /api/experiments` - List experiments
- [ ] `POST /api/experiments` - Create experiment
- [ ] `GET /api/experiments/:id/results` - Get results
- [ ] `POST /api/experiments/:id/promote` - Promote winner
- [ ] `GET /api/predictions` - Task predictions

### Story 12: Learning Dashboard UI

Add learning pages to web dashboard.

**Acceptance Criteria:**
- [ ] Learning overview with key metrics
- [ ] Instruction effectiveness chart
- [ ] Experiment management interface
- [ ] Feedback submission UI
- [ ] Pattern browser
- [ ] Suggestion review interface
- [ ] Prediction tool

### Story 13: Continuous Improvement Metrics

Track overall system improvement.

**Acceptance Criteria:**
- [ ] Track success rate over time
- [ ] Track cost efficiency over time
- [ ] Track completion time trends
- [ ] Track instruction count and quality
- [ ] Track feedback sentiment
- [ ] Weekly/monthly improvement reports

**Improvement Report:**
```
Weekly Improvement Report

Success Rate: 78% → 82% (+4%)
Avg Completion Time: 32m → 28m (-12%)
Cost per Task: $0.45 → $0.38 (-15%)
Active Instructions: 45 (+3 new, -1 deprecated)

Top Improvements:
  1. New instruction "validate API responses" +5% success
  2. Prompt v2 for code-reviewer +8% fix rate
  3. Model selection optimization -20% cost

Areas for Improvement:
  1. Complex refactoring tasks (62% success)
  2. Database migration tasks (68% success)
```

## Definition of Done

- [ ] All stories completed and tested
- [ ] Learning from successes operational
- [ ] Feedback collection working
- [ ] A/B testing framework functional
- [ ] Model selection optimizing
- [ ] Cross-project learning tested
- [ ] Measurable improvement demonstrated
- [ ] Documentation complete
