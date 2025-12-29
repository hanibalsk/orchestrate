import { useState } from 'react';
import type { Message } from '@/api/types';
import { cn, formatDate } from '@/lib/utils';
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from '@/components/ui/collapsible';
import { ChevronDown } from 'lucide-react';

interface MessageItemProps {
  message: Message;
}

export function MessageItem({ message }: MessageItemProps) {
  const roleLabels: Record<string, string> = {
    user: 'User',
    assistant: 'Assistant',
    tool: 'Tool',
    system: 'System',
  };

  const roleClasses: Record<string, string> = {
    user: 'bg-info text-white ml-auto',
    assistant: 'bg-muted',
    tool: 'bg-background border',
    system: 'bg-muted/50 italic text-muted-foreground',
  };

  return (
    <div
      className={cn(
        'rounded-lg p-4 max-w-[85%]',
        roleClasses[message.role] || 'bg-muted'
      )}
    >
      <div className="flex justify-between items-center mb-2 text-xs">
        <span className="font-semibold uppercase">
          {roleLabels[message.role] || message.role}
        </span>
        <span className="opacity-70">{formatDate(message.created_at)}</span>
      </div>

      <div className="whitespace-pre-wrap break-words">{message.content}</div>

      {/* Tool Calls */}
      {message.tool_calls && message.tool_calls.length > 0 && (
        <div className="mt-3 space-y-2">
          {message.tool_calls.map((toolCall) => (
            <ToolCallDisplay key={toolCall.id} toolCall={toolCall} />
          ))}
        </div>
      )}

      {/* Tool Results */}
      {message.tool_results && message.tool_results.length > 0 && (
        <div className="mt-3 space-y-2">
          {message.tool_results.map((result, index) => (
            <ToolResultDisplay key={index} result={result} />
          ))}
        </div>
      )}
    </div>
  );
}

function ToolCallDisplay({
  toolCall,
}: {
  toolCall: { id: string; name: string; input: Record<string, unknown> };
}) {
  const [isOpen, setIsOpen] = useState(false);

  return (
    <Collapsible open={isOpen} onOpenChange={setIsOpen}>
      <CollapsibleTrigger className="flex items-center gap-2 text-sm bg-secondary rounded px-3 py-2 w-full text-left hover:bg-secondary/80">
        <ChevronDown
          className={cn('h-4 w-4 transition-transform', isOpen && 'rotate-180')}
        />
        <span className="font-mono">{toolCall.name}</span>
      </CollapsibleTrigger>
      <CollapsibleContent>
        <pre className="mt-2 p-3 bg-background rounded text-xs overflow-x-auto">
          {JSON.stringify(toolCall.input, null, 2)}
        </pre>
      </CollapsibleContent>
    </Collapsible>
  );
}

function ToolResultDisplay({
  result,
}: {
  result: { tool_call_id: string; content: string; is_error: boolean };
}) {
  const [isOpen, setIsOpen] = useState(false);
  const truncatedContent =
    result.content.length > 200
      ? result.content.slice(0, 200) + '...'
      : result.content;

  return (
    <Collapsible open={isOpen} onOpenChange={setIsOpen}>
      <CollapsibleTrigger
        className={cn(
          'flex items-center gap-2 text-sm rounded px-3 py-2 w-full text-left',
          result.is_error
            ? 'bg-danger/20 border-l-2 border-danger'
            : 'bg-secondary hover:bg-secondary/80'
        )}
      >
        <ChevronDown
          className={cn('h-4 w-4 transition-transform', isOpen && 'rotate-180')}
        />
        <span className="font-mono text-xs truncate">
          {result.is_error ? 'Error: ' : ''}
          {truncatedContent}
        </span>
      </CollapsibleTrigger>
      <CollapsibleContent>
        <pre className="mt-2 p-3 bg-background rounded text-xs overflow-x-auto whitespace-pre-wrap">
          {result.content}
        </pre>
      </CollapsibleContent>
    </Collapsible>
  );
}
