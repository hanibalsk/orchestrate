import { useState, useEffect } from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../components/ui/card';
import { Button } from '../components/ui/button';
import { Badge } from '../components/ui/badge';
import { FileText, Book, GitCommit, FileCode, ExternalLink } from 'lucide-react';

interface DocItem {
  type: 'api' | 'readme' | 'changelog' | 'adr';
  title: string;
  description: string;
  path: string;
  lastUpdated?: string;
  status?: string;
}

export function DocsList() {
  const [docs, setDocs] = useState<DocItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [coverage, setCoverage] = useState<{
    percentage: number;
    totalItems: number;
    documentedItems: number;
    issues: number;
  } | null>(null);

  useEffect(() => {
    loadDocumentation();
    loadCoverage();
  }, []);

  const loadDocumentation = async () => {
    try {
      // Mock data for now - would call API in real implementation
      const mockDocs: DocItem[] = [
        {
          type: 'api',
          title: 'Orchestrate API',
          description: 'OpenAPI 3.0 specification for all REST endpoints',
          path: '/api/docs',
          lastUpdated: new Date().toISOString(),
        },
        {
          type: 'readme',
          title: 'Project README',
          description: 'Main project documentation and getting started guide',
          path: '/README.md',
          lastUpdated: new Date().toISOString(),
        },
        {
          type: 'changelog',
          title: 'Changelog',
          description: 'Version history and release notes',
          path: '/CHANGELOG.md',
          lastUpdated: new Date().toISOString(),
        },
      ];
      setDocs(mockDocs);
    } catch (error) {
      console.error('Failed to load documentation:', error);
    } finally {
      setLoading(false);
    }
  };

  const loadCoverage = async () => {
    try {
      // Mock data - would call validation API
      setCoverage({
        percentage: 85.5,
        totalItems: 200,
        documentedItems: 171,
        issues: 5,
      });
    } catch (error) {
      console.error('Failed to load coverage:', error);
    }
  };

  const getIcon = (type: string) => {
    switch (type) {
      case 'api':
        return <FileCode className="h-5 w-5" />;
      case 'readme':
        return <Book className="h-5 w-5" />;
      case 'changelog':
        return <GitCommit className="h-5 w-5" />;
      case 'adr':
        return <FileText className="h-5 w-5" />;
      default:
        return <FileText className="h-5 w-5" />;
    }
  };

  const getTypeColor = (type: string) => {
    switch (type) {
      case 'api':
        return 'bg-blue-500/10 text-blue-500 hover:bg-blue-500/20';
      case 'readme':
        return 'bg-green-500/10 text-green-500 hover:bg-green-500/20';
      case 'changelog':
        return 'bg-purple-500/10 text-purple-500 hover:bg-purple-500/20';
      case 'adr':
        return 'bg-orange-500/10 text-orange-500 hover:bg-orange-500/20';
      default:
        return 'bg-gray-500/10 text-gray-500 hover:bg-gray-500/20';
    }
  };

  if (loading) {
    return (
      <div className="space-y-6">
        <h1 className="text-3xl font-bold">Documentation</h1>
        <div className="text-muted-foreground">Loading...</div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold">Documentation</h1>
          <p className="text-muted-foreground">
            Manage and view project documentation
          </p>
        </div>
        <div className="flex gap-2">
          <Button variant="outline" onClick={() => window.location.href = '/docs/validate'}>
            Validate Coverage
          </Button>
          <Button onClick={() => window.location.href = '/docs/generate'}>
            Generate Docs
          </Button>
        </div>
      </div>

      {coverage && (
        <Card>
          <CardHeader>
            <CardTitle>Documentation Coverage</CardTitle>
            <CardDescription>
              Overall documentation coverage across the codebase
            </CardDescription>
          </CardHeader>
          <CardContent>
            <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
              <div className="space-y-1">
                <div className="text-2xl font-bold">{coverage.percentage.toFixed(1)}%</div>
                <div className="text-sm text-muted-foreground">Coverage</div>
              </div>
              <div className="space-y-1">
                <div className="text-2xl font-bold">{coverage.totalItems}</div>
                <div className="text-sm text-muted-foreground">Total Items</div>
              </div>
              <div className="space-y-1">
                <div className="text-2xl font-bold">{coverage.documentedItems}</div>
                <div className="text-sm text-muted-foreground">Documented</div>
              </div>
              <div className="space-y-1">
                <div className="text-2xl font-bold text-yellow-600">{coverage.issues}</div>
                <div className="text-sm text-muted-foreground">Issues</div>
              </div>
            </div>
            <div className="mt-4">
              <div className="h-2 bg-muted rounded-full overflow-hidden">
                <div
                  className="h-full bg-primary transition-all"
                  style={{ width: `${coverage.percentage}%` }}
                />
              </div>
            </div>
          </CardContent>
        </Card>
      )}

      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        {docs.map((doc, index) => (
          <Card key={index} className="hover:shadow-lg transition-shadow">
            <CardHeader>
              <div className="flex items-start justify-between">
                <div className={`p-2 rounded-lg ${getTypeColor(doc.type)}`}>
                  {getIcon(doc.type)}
                </div>
                <Badge variant="outline">{doc.type.toUpperCase()}</Badge>
              </div>
              <CardTitle className="mt-4">{doc.title}</CardTitle>
              <CardDescription>{doc.description}</CardDescription>
            </CardHeader>
            <CardContent>
              <div className="flex items-center justify-between">
                {doc.lastUpdated && (
                  <div className="text-sm text-muted-foreground">
                    Updated {new Date(doc.lastUpdated).toLocaleDateString()}
                  </div>
                )}
                <Button variant="ghost" size="sm" asChild>
                  <a href={doc.path} target="_blank" rel="noopener noreferrer">
                    <ExternalLink className="h-4 w-4 mr-1" />
                    View
                  </a>
                </Button>
              </div>
            </CardContent>
          </Card>
        ))}
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Architecture Decision Records</CardTitle>
          <CardDescription>
            View and manage architectural decisions
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="flex items-center justify-between">
            <p className="text-sm text-muted-foreground">
              No ADRs found. Create your first ADR to document architectural decisions.
            </p>
            <Button onClick={() => window.location.href = '/docs/adr/new'}>
              Create ADR
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
