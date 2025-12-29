import { useState } from 'react';
import { Rocket } from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';

interface DeployButtonProps {
  environment: string;
  onDeploy: (version: string, strategy?: string) => void;
  disabled?: boolean;
}

export function DeployButton({ environment, onDeploy, disabled }: DeployButtonProps) {
  const [open, setOpen] = useState(false);
  const [version, setVersion] = useState('');
  const [strategy, setStrategy] = useState<string>('');

  const handleDeploy = () => {
    if (version.trim()) {
      onDeploy(version.trim(), strategy || undefined);
      setOpen(false);
      setVersion('');
      setStrategy('');
    }
  };

  return (
    <>
      <Button onClick={() => setOpen(true)} disabled={disabled} variant="default">
        <Rocket className="mr-2 h-4 w-4" />
        Deploy
      </Button>

      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Deploy to {environment}</DialogTitle>
            <DialogDescription>
              Deploy a specific version to the {environment} environment.
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label htmlFor="version">Version</Label>
              <Input
                id="version"
                placeholder="e.g., 1.0.0 or v1.0.0"
                value={version}
                onChange={(e) => setVersion(e.target.value)}
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="strategy">Deployment Strategy (Optional)</Label>
              <select
                id="strategy"
                className="flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-sm transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
                value={strategy}
                onChange={(e) => setStrategy(e.target.value)}
              >
                <option value="">Default</option>
                <option value="rolling">Rolling</option>
                <option value="blue-green">Blue-Green</option>
                <option value="canary">Canary</option>
                <option value="recreate">Recreate</option>
              </select>
            </div>
          </div>

          <DialogFooter>
            <Button variant="outline" onClick={() => setOpen(false)}>
              Cancel
            </Button>
            <Button onClick={handleDeploy} disabled={!version.trim()}>
              Deploy
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
