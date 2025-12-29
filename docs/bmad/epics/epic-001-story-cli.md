# Epic 001: Story CLI Commands

Implement CLI commands for managing stories.

## Stories

## Story 1: List Stories Command
Implement `orchestrate story list` command to show all stories.

- [ ] Add StoryAction enum with List variant
- [ ] Implement list_stories handler
- [ ] Support filtering by epic ID
- [ ] Support filtering by status

## Story 2: Show Story Command
Implement `orchestrate story show <id>` command.

- [ ] Add Show variant to StoryAction
- [ ] Fetch story from database
- [ ] Display story details and acceptance criteria

## Story 3: Create Story Command
Implement `orchestrate story create` command.

- [ ] Add Create variant to StoryAction
- [ ] Accept epic_id, title, description
- [ ] Save to database
