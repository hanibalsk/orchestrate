import { useState } from 'react';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { sendMessage } from '@/api/agents';
import { Button } from '@/components/ui/button';
import { Send } from 'lucide-react';

interface MessageInputProps {
  agentId: string;
  disabled?: boolean;
}

export function MessageInput({ agentId, disabled }: MessageInputProps) {
  const [content, setContent] = useState('');
  const queryClient = useQueryClient();

  const mutation = useMutation({
    mutationFn: (message: string) => sendMessage(agentId, message),
    onSuccess: () => {
      setContent('');
      queryClient.invalidateQueries({ queryKey: ['agent', agentId, 'messages'] });
    },
  });

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!content.trim() || disabled) return;
    mutation.mutate(content.trim());
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
      handleSubmit(e);
    }
  };

  return (
    <form onSubmit={handleSubmit} className="border-t p-4">
      <div className="flex gap-2">
        <textarea
          className="flex-1 min-h-[80px] rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring resize-none"
          placeholder={
            disabled
              ? 'Agent is not accepting input'
              : 'Type a message... (Ctrl+Enter to send)'
          }
          value={content}
          onChange={(e) => setContent(e.target.value)}
          onKeyDown={handleKeyDown}
          disabled={disabled || mutation.isPending}
        />
        <Button
          type="submit"
          disabled={disabled || mutation.isPending || !content.trim()}
          className="self-end"
        >
          <Send className="h-4 w-4" />
        </Button>
      </div>
    </form>
  );
}
