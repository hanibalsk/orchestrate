import { useParams, useNavigate } from 'react-router-dom';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  getAgent,
  getMessages,
  pauseAgent,
  resumeAgent,
  terminateAgent,
} from '@/api/agents';
import { useWebSocket } from '@/hooks/useWebSocket';
import { AgentStateBadge, AgentTypeBadge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { MessageList } from '@/components/chat/MessageList';
import { MessageInput } from '@/components/chat/MessageInput';
import { formatDate } from '@/lib/utils';
import { ArrowLeft, Pause, Play, XCircle } from 'lucide-react';

export function AgentDetail() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const queryClient = useQueryClient();

  const { data: agent, isLoading: agentLoading } = useQuery({
    queryKey: ['agent', id],
    queryFn: () => getAgent(id!),
    enabled: !!id,
  });

  const { data: messages = [], isLoading: messagesLoading } = useQuery({
    queryKey: ['agent', id, 'messages'],
    queryFn: () => getMessages(id!),
    enabled: !!id,
    refetchInterval: 5000,
  });

  // WebSocket for real-time updates
  useWebSocket({
    agentId: id,
    onAgentStateChange: (agentId, state) => {
      if (agentId === id) {
        queryClient.setQueryData(['agent', id], (old: typeof agent) =>
          old ? { ...old, state } : old
        );
      }
    },
    onNewMessage: (agentId) => {
      if (agentId === id) {
        queryClient.invalidateQueries({ queryKey: ['agent', id, 'messages'] });
      }
    },
  });

  const pauseMutation = useMutation({
    mutationFn: () => pauseAgent(id!),
    onSuccess: (data) =>
      queryClient.setQueryData(['agent', id], data),
  });

  const resumeMutation = useMutation({
    mutationFn: () => resumeAgent(id!),
    onSuccess: (data) =>
      queryClient.setQueryData(['agent', id], data),
  });

  const terminateMutation = useMutation({
    mutationFn: () => terminateAgent(id!),
    onSuccess: (data) =>
      queryClient.setQueryData(['agent', id], data),
  });

  if (agentLoading) {
    return (
      <div className="text-center py-12 text-muted-foreground">
        Loading agent...
      </div>
    );
  }

  if (!agent) {
    return (
      <div className="text-center py-12">
        <p className="text-muted-foreground mb-4">Agent not found</p>
        <Button variant="outline" onClick={() => navigate('/agents')}>
          <ArrowLeft className="h-4 w-4 mr-2" />
          Back to Agents
        </Button>
      </div>
    );
  }

  const canPause = ['running', 'waiting_for_input', 'waiting_for_external'].includes(
    agent.state
  );
  const canResume = agent.state === 'paused';
  const canTerminate = !['completed', 'failed', 'terminated'].includes(
    agent.state
  );
  const canSendMessage = ['running', 'waiting_for_input', 'paused'].includes(
    agent.state
  );

  return (
    <div className="space-y-6 max-w-4xl">
      {/* Header */}
      <div className="flex items-start justify-between">
        <div>
          <Button
            variant="ghost"
            size="sm"
            className="mb-2"
            onClick={() => navigate('/agents')}
          >
            <ArrowLeft className="h-4 w-4 mr-1" />
            Back
          </Button>
          <div className="flex items-center gap-3">
            <h1 className="text-2xl font-bold">Agent Details</h1>
            <AgentTypeBadge type={agent.agent_type} />
            <AgentStateBadge state={agent.state} />
          </div>
          <p className="text-sm text-muted-foreground font-mono mt-1">
            {agent.id}
          </p>
        </div>

        {/* Controls */}
        <div className="flex gap-2">
          {canPause && (
            <Button
              variant="outline"
              onClick={() => pauseMutation.mutate()}
              disabled={pauseMutation.isPending}
            >
              <Pause className="h-4 w-4 mr-2" />
              Pause
            </Button>
          )}
          {canResume && (
            <Button
              variant="success"
              onClick={() => resumeMutation.mutate()}
              disabled={resumeMutation.isPending}
            >
              <Play className="h-4 w-4 mr-2" />
              Resume
            </Button>
          )}
          {canTerminate && (
            <Button
              variant="destructive"
              onClick={() => terminateMutation.mutate()}
              disabled={terminateMutation.isPending}
            >
              <XCircle className="h-4 w-4 mr-2" />
              Terminate
            </Button>
          )}
        </div>
      </div>

      {/* Agent Info */}
      <Card>
        <CardHeader>
          <CardTitle>Task</CardTitle>
        </CardHeader>
        <CardContent>
          <p className="whitespace-pre-wrap">{agent.task}</p>
          <div className="flex gap-6 mt-4 text-sm text-muted-foreground">
            <div>
              <strong>Created:</strong> {formatDate(agent.created_at)}
            </div>
            <div>
              <strong>Updated:</strong> {formatDate(agent.updated_at)}
            </div>
          </div>
          {agent.error_message && (
            <div className="mt-4 p-4 bg-danger/10 border-l-4 border-danger rounded">
              <p className="font-medium text-danger">Error</p>
              <p className="text-sm mt-1">{agent.error_message}</p>
            </div>
          )}
        </CardContent>
      </Card>

      {/* Chat */}
      <Card>
        <CardHeader>
          <CardTitle>Conversation</CardTitle>
        </CardHeader>
        <CardContent className="p-0">
          {messagesLoading ? (
            <div className="p-8 text-center text-muted-foreground">
              Loading messages...
            </div>
          ) : (
            <MessageList messages={messages} />
          )}
          <MessageInput agentId={id!} disabled={!canSendMessage} />
        </CardContent>
      </Card>
    </div>
  );
}
