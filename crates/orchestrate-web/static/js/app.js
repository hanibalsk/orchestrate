// Orchestrate Web UI JavaScript

// WebSocket connection for real-time updates
let ws = null;
let reconnectAttempts = 0;
const maxReconnectAttempts = 5;

function initWebSocket(agentId = null) {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const wsUrl = `${protocol}//${window.location.host}/ws`;

    ws = new WebSocket(wsUrl);

    ws.onopen = function() {
        console.log('WebSocket connected');
        reconnectAttempts = 0;

        // Subscribe to specific agent if provided
        if (agentId) {
            ws.send(JSON.stringify({
                type: 'subscribe',
                agent_id: agentId
            }));
        }
    };

    ws.onmessage = function(event) {
        try {
            const data = JSON.parse(event.data);
            handleWebSocketMessage(data);
        } catch (e) {
            console.error('Failed to parse WebSocket message:', e);
        }
    };

    ws.onclose = function() {
        console.log('WebSocket disconnected');

        // Attempt to reconnect
        if (reconnectAttempts < maxReconnectAttempts) {
            reconnectAttempts++;
            const delay = Math.min(1000 * Math.pow(2, reconnectAttempts), 30000);
            console.log(`Reconnecting in ${delay}ms...`);
            setTimeout(() => initWebSocket(agentId), delay);
        }
    };

    ws.onerror = function(error) {
        console.error('WebSocket error:', error);
    };
}

function handleWebSocketMessage(data) {
    switch (data.type) {
        case 'new_message':
            appendMessage(data.message);
            break;
        case 'state_changed':
            updateAgentState(data.agent_id, data.new_state);
            break;
        case 'agent_created':
            // Reload page to show new agent
            window.location.reload();
            break;
        case 'ping':
            // Respond to keep-alive
            if (ws && ws.readyState === WebSocket.OPEN) {
                ws.send(JSON.stringify({ type: 'pong' }));
            }
            break;
        default:
            console.log('Unknown message type:', data.type);
    }
}

function appendMessage(message) {
    const messagesContainer = document.getElementById('messages');
    if (!messagesContainer) return;

    const messageEl = document.createElement('div');
    messageEl.className = `message message-${message.role}`;

    const headerEl = document.createElement('div');
    headerEl.className = 'message-header';
    headerEl.innerHTML = `
        <span class="message-role">${message.role}</span>
        <span class="message-time">${message.created_at || 'now'}</span>
    `;

    const contentEl = document.createElement('div');
    contentEl.className = 'message-content';

    if (message.role === 'tool' && message.tool_results) {
        contentEl.innerHTML = formatToolResults(message.tool_results);
    } else if (message.role === 'assistant' && message.tool_calls) {
        contentEl.innerHTML = `
            <div class="assistant-content">${escapeHtml(message.content || '')}</div>
            <div class="tool-calls">${formatToolCalls(message.tool_calls)}</div>
        `;
    } else {
        contentEl.innerHTML = `<div class="text-content">${escapeHtml(message.content || '')}</div>`;
    }

    messageEl.appendChild(headerEl);
    messageEl.appendChild(contentEl);
    messagesContainer.appendChild(messageEl);

    // Auto-scroll to bottom
    messagesContainer.scrollTop = messagesContainer.scrollHeight;
}

function formatToolResults(results) {
    return results.map(result => `
        <div class="tool-result ${result.is_error ? 'tool-error' : ''}">
            <div class="tool-id">Tool: ${truncate(result.tool_call_id, 20)}</div>
            <pre>${escapeHtml(truncate(result.content, 500))}</pre>
        </div>
    `).join('');
}

function formatToolCalls(calls) {
    return calls.map(call => `
        <div class="tool-call">
            <details>
                <summary>Tool: ${escapeHtml(call.name)}</summary>
                <pre>${escapeHtml(call.input || '')}</pre>
            </details>
        </div>
    `).join('');
}

function updateAgentState(agentId, newState) {
    // Update state badge on detail page
    const stateBadge = document.querySelector('.agent-status .badge');
    if (stateBadge) {
        stateBadge.className = `badge badge-state-${newState.toLowerCase()} badge-lg`;
        stateBadge.textContent = newState;
    }

    // Update controls visibility
    updateControls(newState);

    // Update state in agent list table
    const row = document.querySelector(`tr[data-agent-id="${agentId}"]`);
    if (row) {
        const stateCell = row.querySelector('.badge-state');
        if (stateCell) {
            stateCell.className = `badge badge-state-${newState.toLowerCase()}`;
            stateCell.textContent = newState;
        }
    }
}

function updateControls(state) {
    const controlsContainer = document.querySelector('.agent-controls');
    if (!controlsContainer) return;

    // Hide/show buttons based on state
    const pauseBtn = controlsContainer.querySelector('[action*="pause"]');
    const resumeBtn = controlsContainer.querySelector('[action*="resume"]');
    const terminateBtn = controlsContainer.querySelector('[action*="terminate"]');
    const messageForm = document.querySelector('.message-form');

    const isActive = ['Running', 'WaitingForInput', 'WaitingForExternal'].includes(state);
    const isPaused = state === 'Paused';
    const isEnded = ['Completed', 'Failed', 'Terminated'].includes(state);

    if (pauseBtn) pauseBtn.parentElement.style.display = isActive ? 'inline' : 'none';
    if (resumeBtn) resumeBtn.parentElement.style.display = isPaused ? 'inline' : 'none';
    if (terminateBtn) terminateBtn.parentElement.style.display = isEnded ? 'none' : 'inline';
    if (messageForm) messageForm.style.display = (isActive || isPaused) ? 'block' : 'none';
}

// Utility functions
function escapeHtml(text) {
    if (!text) return '';
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

function truncate(text, length) {
    if (!text) return '';
    if (text.length <= length) return text;
    return text.substring(0, length) + '...';
}

// Auto-refresh for dashboard stats
function initDashboardRefresh(intervalMs = 30000) {
    setInterval(async () => {
        try {
            const response = await fetch('/api/status');
            if (response.ok) {
                const status = await response.json();
                updateDashboardStats(status);
            }
        } catch (e) {
            console.error('Failed to refresh dashboard:', e);
        }
    }, intervalMs);
}

function updateDashboardStats(status) {
    const statsMap = {
        'total': status.total_agents,
        'running': status.running_agents,
        'paused': status.paused_agents,
        'completed': status.completed_agents
    };

    for (const [key, value] of Object.entries(statsMap)) {
        const el = document.querySelector(`.stat-${key} .stat-value, .stat-card:nth-child(${getStatIndex(key)}) .stat-value`);
        if (el) el.textContent = value;
    }
}

function getStatIndex(key) {
    const indexMap = { 'total': 1, 'running': 2, 'paused': 3, 'completed': 4 };
    return indexMap[key] || 1;
}

// Form handling with loading states
document.addEventListener('submit', function(e) {
    const form = e.target;
    if (form.tagName !== 'FORM') return;

    const submitBtn = form.querySelector('button[type="submit"]');
    if (submitBtn) {
        submitBtn.disabled = true;
        submitBtn.dataset.originalText = submitBtn.textContent;
        submitBtn.textContent = 'Loading...';
    }
});

// Keyboard shortcuts
document.addEventListener('keydown', function(e) {
    // Ctrl+Enter to submit message
    if (e.ctrlKey && e.key === 'Enter') {
        const messageInput = document.getElementById('messageInput');
        if (messageInput && document.activeElement === messageInput) {
            const form = messageInput.closest('form');
            if (form) form.submit();
        }
    }
});

// Initialize on page load
document.addEventListener('DOMContentLoaded', function() {
    // Initialize WebSocket if on dashboard or agent list
    const path = window.location.pathname;
    if (path === '/' || path === '/agents') {
        initWebSocket();
        if (path === '/') {
            initDashboardRefresh();
        }
    }
});
