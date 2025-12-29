import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
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
import { Textarea } from '@/components/ui/textarea';
import { listReleases, createRelease, publishRelease } from '@/api/deployments';
import { formatDistanceToNow } from '@/lib/time';
import { Plus, ExternalLink, Send } from 'lucide-react';

export function Releases() {
  const queryClient = useQueryClient();
  const [createDialogOpen, setCreateDialogOpen] = useState(false);
  const [publishDialogOpen, setPublishDialogOpen] = useState(false);
  const [selectedRelease, setSelectedRelease] = useState<string | null>(null);

  const [version, setVersion] = useState('');
  const [tagName, setTagName] = useState('');
  const [changelog, setChangelog] = useState('');

  const { data: releases = [], isLoading } = useQuery({
    queryKey: ['releases'],
    queryFn: listReleases,
  });

  const createMutation = useMutation({
    mutationFn: createRelease,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['releases'] });
      setCreateDialogOpen(false);
      setVersion('');
      setTagName('');
      setChangelog('');
    },
  });

  const publishMutation = useMutation({
    mutationFn: publishRelease,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['releases'] });
      setPublishDialogOpen(false);
      setSelectedRelease(null);
    },
  });

  const handleCreate = () => {
    if (version.trim() && tagName.trim()) {
      createMutation.mutate({
        version: version.trim(),
        tag_name: tagName.trim(),
        changelog: changelog.trim() || undefined,
      });
    }
  };

  const handlePublish = () => {
    if (selectedRelease) {
      publishMutation.mutate(selectedRelease);
    }
  };

  const openPublishDialog = (releaseVersion: string) => {
    setSelectedRelease(releaseVersion);
    setPublishDialogOpen(true);
  };

  if (isLoading) {
    return (
      <div className="space-y-8">
        <div className="flex items-center justify-between">
          <h1 className="text-3xl font-bold">Releases</h1>
        </div>
        <div className="text-center py-12 text-muted-foreground">Loading...</div>
      </div>
    );
  }

  return (
    <div className="space-y-8">
      <div className="flex items-center justify-between">
        <h1 className="text-3xl font-bold">Releases</h1>
        <Button onClick={() => setCreateDialogOpen(true)}>
          <Plus className="mr-2 h-4 w-4" />
          Create Release
        </Button>
      </div>

      {releases.length === 0 ? (
        <Card>
          <CardContent className="py-12">
            <div className="text-center text-muted-foreground">
              <p className="mb-4">No releases yet</p>
              <Button onClick={() => setCreateDialogOpen(true)}>
                Create your first release
              </Button>
            </div>
          </CardContent>
        </Card>
      ) : (
        <div className="grid gap-4">
          {releases.map((release) => (
            <Card key={release.id}>
              <CardHeader>
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-3">
                    <CardTitle>v{release.version}</CardTitle>
                    {release.published ? (
                      <Badge variant="success">Published</Badge>
                    ) : (
                      <Badge variant="secondary">Draft</Badge>
                    )}
                  </div>
                  <div className="flex items-center gap-2">
                    {!release.published && (
                      <Button
                        variant="default"
                        size="sm"
                        onClick={() => openPublishDialog(release.version)}
                        disabled={publishMutation.isPending}
                      >
                        <Send className="mr-2 h-4 w-4" />
                        Publish
                      </Button>
                    )}
                    {release.github_release_url && (
                      <Button
                        variant="outline"
                        size="sm"
                        asChild
                      >
                        <a
                          href={release.github_release_url}
                          target="_blank"
                          rel="noopener noreferrer"
                        >
                          <ExternalLink className="mr-2 h-4 w-4" />
                          View on GitHub
                        </a>
                      </Button>
                    )}
                  </div>
                </div>
              </CardHeader>
              <CardContent>
                <div className="space-y-3">
                  <div className="text-sm text-muted-foreground">
                    Tag: <span className="font-mono">{release.tag_name}</span>
                  </div>

                  <div className="text-sm text-muted-foreground">
                    Created {formatDistanceToNow(release.created_at)}
                    {release.published_at && (
                      <> • Published {formatDistanceToNow(release.published_at)}</>
                    )}
                  </div>

                  {release.changelog && (
                    <div className="mt-4 p-4 bg-muted rounded-md">
                      <div className="text-sm font-semibold mb-2">Changelog</div>
                      <pre className="text-xs whitespace-pre-wrap text-muted-foreground">
                        {release.changelog}
                      </pre>
                    </div>
                  )}
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      )}

      <Dialog open={createDialogOpen} onOpenChange={setCreateDialogOpen}>
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>Create Release</DialogTitle>
            <DialogDescription>
              Create a new release with version information and changelog.
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label htmlFor="version">Version</Label>
              <Input
                id="version"
                placeholder="e.g., 1.0.0"
                value={version}
                onChange={(e) => setVersion(e.target.value)}
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="tag">Git Tag</Label>
              <Input
                id="tag"
                placeholder="e.g., v1.0.0"
                value={tagName}
                onChange={(e) => setTagName(e.target.value)}
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="changelog">Changelog (Optional)</Label>
              <Textarea
                id="changelog"
                placeholder="## What's Changed&#10;- Feature 1&#10;- Feature 2"
                value={changelog}
                onChange={(e) => setChangelog(e.target.value)}
                rows={8}
                className="font-mono text-sm"
              />
            </div>
          </div>

          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setCreateDialogOpen(false)}
              disabled={createMutation.isPending}
            >
              Cancel
            </Button>
            <Button
              onClick={handleCreate}
              disabled={!version.trim() || !tagName.trim() || createMutation.isPending}
            >
              {createMutation.isPending ? 'Creating...' : 'Create Release'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={publishDialogOpen} onOpenChange={setPublishDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Publish Release</DialogTitle>
            <DialogDescription>
              Are you sure you want to publish release v{selectedRelease}? This will create a
              GitHub release and make it publicly visible.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setPublishDialogOpen(false)}
              disabled={publishMutation.isPending}
            >
              Cancel
            </Button>
            <Button
              onClick={handlePublish}
              disabled={publishMutation.isPending}
            >
              {publishMutation.isPending ? 'Publishing...' : 'Publish'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
