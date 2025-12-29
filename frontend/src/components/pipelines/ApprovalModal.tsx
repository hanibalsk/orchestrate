import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { CheckCircle2, XCircle } from 'lucide-react';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
import { approveApproval, rejectApproval, listPendingApprovals } from '@/api/pipelines';

interface ApprovalModalProps {
  approvalId: number;
  onClose: () => void;
}

export function ApprovalModal({ approvalId, onClose }: ApprovalModalProps) {
  const queryClient = useQueryClient();
  const [approver, setApprover] = useState('');
  const [comment, setComment] = useState('');

  const { data: approvals = [] } = useQuery({
    queryKey: ['approvals'],
    queryFn: listPendingApprovals,
  });

  const approval = approvals.find((a) => a.id === approvalId);

  const approveMutation = useMutation({
    mutationFn: () => approveApproval(approvalId, { approver, comment }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['approvals'] });
      queryClient.invalidateQueries({ queryKey: ['pipeline-run'] });
      queryClient.invalidateQueries({ queryKey: ['pipeline-stages'] });
      onClose();
    },
  });

  const rejectMutation = useMutation({
    mutationFn: () => rejectApproval(approvalId, { approver, comment }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['approvals'] });
      queryClient.invalidateQueries({ queryKey: ['pipeline-run'] });
      queryClient.invalidateQueries({ queryKey: ['pipeline-stages'] });
      onClose();
    },
  });

  const handleApprove = () => {
    if (!approver.trim()) {
      alert('Please enter your name or email');
      return;
    }
    approveMutation.mutate();
  };

  const handleReject = () => {
    if (!approver.trim()) {
      alert('Please enter your name or email');
      return;
    }
    if (!comment.trim()) {
      if (!window.confirm('Are you sure you want to reject without a comment?')) {
        return;
      }
    }
    rejectMutation.mutate();
  };

  if (!approval) {
    return null;
  }

  const requiredApprovers = approval.required_approvers
    .split(',')
    .map((a) => a.trim())
    .filter((a) => a.length > 0);

  const isPending = approveMutation.isPending || rejectMutation.isPending;

  return (
    <Dialog open={true} onOpenChange={onClose}>
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle>Pipeline Approval Request</DialogTitle>
          <DialogDescription>
            Review and approve or reject this pipeline stage
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          {/* Approval Status */}
          <div className="grid grid-cols-2 gap-4 p-4 border rounded-md bg-muted">
            <div>
              <div className="text-sm text-muted-foreground">Status</div>
              <div className="mt-1">
                <Badge variant="warning">{approval.status}</Badge>
              </div>
            </div>
            <div>
              <div className="text-sm text-muted-foreground">Required Approvals</div>
              <div className="mt-1 font-semibold">
                {approval.approval_count} / {approval.required_count}
              </div>
            </div>
          </div>

          {/* Required Approvers */}
          {requiredApprovers.length > 0 && (
            <div>
              <label className="text-sm font-medium">Required Approvers</label>
              <div className="mt-2 flex flex-wrap gap-2">
                {requiredApprovers.map((approver) => (
                  <Badge key={approver} variant="secondary">
                    {approver}
                  </Badge>
                ))}
              </div>
            </div>
          )}

          {/* Approver Input */}
          <div>
            <label className="text-sm font-medium" htmlFor="approver">
              Your Name or Email
            </label>
            <Input
              id="approver"
              type="text"
              className="mt-2"
              placeholder="your.email@example.com"
              value={approver}
              onChange={(e) => setApprover(e.target.value)}
              disabled={isPending}
            />
          </div>

          {/* Comment */}
          <div>
            <label className="text-sm font-medium" htmlFor="comment">
              Comment (optional)
            </label>
            <textarea
              id="comment"
              className="mt-2 w-full px-3 py-2 border rounded-md"
              rows={3}
              placeholder="Add any comments about this approval decision..."
              value={comment}
              onChange={(e) => setComment(e.target.value)}
              disabled={isPending}
            />
          </div>

          {/* Timeout Info */}
          {approval.timeout_at && (
            <div className="p-3 border rounded-md bg-yellow-50 dark:bg-yellow-950 text-sm">
              <strong>Timeout: </strong>
              {new Date(approval.timeout_at).toLocaleString()}
              {approval.timeout_action && (
                <span className="ml-2">
                  (Action: {approval.timeout_action})
                </span>
              )}
            </div>
          )}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={onClose} disabled={isPending}>
            Cancel
          </Button>
          <Button
            variant="destructive"
            onClick={handleReject}
            disabled={isPending}
          >
            <XCircle className="mr-2 h-4 w-4" />
            {rejectMutation.isPending ? 'Rejecting...' : 'Reject'}
          </Button>
          <Button
            variant="default"
            onClick={handleApprove}
            disabled={isPending}
          >
            <CheckCircle2 className="mr-2 h-4 w-4" />
            {approveMutation.isPending ? 'Approving...' : 'Approve'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
