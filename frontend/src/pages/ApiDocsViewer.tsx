import { useState, useEffect } from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../components/ui/card';
import { Badge } from '../components/ui/badge';
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '../components/ui/collapsible';
import { ChevronDown, ChevronRight } from 'lucide-react';

interface ApiEndpoint {
  method: string;
  path: string;
  summary?: string;
  description?: string;
  tags: string[];
  parameters: ApiParameter[];
}

interface ApiParameter {
  name: string;
  location: 'path' | 'query' | 'header' | 'cookie';
  required: boolean;
  description?: string;
  schemaType: string;
}

export function ApiDocsViewer() {
  const [endpoints, setEndpoints] = useState<ApiEndpoint[]>([]);
  const [selectedTag, setSelectedTag] = useState<string>('all');
  const [loading, setLoading] = useState(true);
  const [expandedEndpoints, setExpandedEndpoints] = useState<Set<string>>(new Set());

  useEffect(() => {
    loadApiDocs();
  }, []);

  const loadApiDocs = async () => {
    try {
      // Mock data - would call API endpoint
      const mockEndpoints: ApiEndpoint[] = [
        {
          method: 'GET',
          path: '/api/agents',
          summary: 'List all agents',
          description: 'Returns a list of all agents in the system',
          tags: ['agents'],
          parameters: [
            {
              name: 'status',
              location: 'query',
              required: false,
              description: 'Filter by agent status',
              schemaType: 'string',
            },
          ],
        },
        {
          method: 'GET',
          path: '/api/agents/{id}',
          summary: 'Get agent by ID',
          description: 'Returns a single agent by ID',
          tags: ['agents'],
          parameters: [
            {
              name: 'id',
              location: 'path',
              required: true,
              description: 'Agent ID',
              schemaType: 'string',
            },
          ],
        },
        {
          method: 'POST',
          path: '/api/agents',
          summary: 'Create agent',
          description: 'Creates a new agent',
          tags: ['agents'],
          parameters: [],
        },
        {
          method: 'GET',
          path: '/api/pipelines',
          summary: 'List all pipelines',
          description: 'Returns a list of all pipelines',
          tags: ['pipelines'],
          parameters: [],
        },
        {
          method: 'POST',
          path: '/api/pipelines/{name}/run',
          summary: 'Run pipeline',
          description: 'Triggers a pipeline execution',
          tags: ['pipelines'],
          parameters: [
            {
              name: 'name',
              location: 'path',
              required: true,
              description: 'Pipeline name',
              schemaType: 'string',
            },
          ],
        },
      ];
      setEndpoints(mockEndpoints);
    } catch (error) {
      console.error('Failed to load API docs:', error);
    } finally {
      setLoading(false);
    }
  };

  const toggleEndpoint = (key: string) => {
    setExpandedEndpoints((prev) => {
      const next = new Set(prev);
      if (next.has(key)) {
        next.delete(key);
      } else {
        next.add(key);
      }
      return next;
    });
  };

  const getMethodColor = (method: string) => {
    switch (method.toUpperCase()) {
      case 'GET':
        return 'bg-blue-500';
      case 'POST':
        return 'bg-green-500';
      case 'PUT':
        return 'bg-yellow-500';
      case 'PATCH':
        return 'bg-orange-500';
      case 'DELETE':
        return 'bg-red-500';
      default:
        return 'bg-gray-500';
    }
  };

  const getAllTags = () => {
    const tags = new Set<string>();
    endpoints.forEach((endpoint) => {
      endpoint.tags.forEach((tag) => tags.add(tag));
    });
    return ['all', ...Array.from(tags)];
  };

  const filteredEndpoints = selectedTag === 'all'
    ? endpoints
    : endpoints.filter((e) => e.tags.includes(selectedTag));

  if (loading) {
    return (
      <div className="space-y-6">
        <h1 className="text-3xl font-bold">API Documentation</h1>
        <div className="text-muted-foreground">Loading...</div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold">API Documentation</h1>
        <p className="text-muted-foreground">
          OpenAPI 3.0 specification for Orchestrate REST API
        </p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Filter by Tag</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="flex flex-wrap gap-2">
            {getAllTags().map((tag) => (
              <Badge
                key={tag}
                variant={selectedTag === tag ? 'default' : 'outline'}
                className="cursor-pointer"
                onClick={() => setSelectedTag(tag)}
              >
                {tag}
              </Badge>
            ))}
          </div>
        </CardContent>
      </Card>

      <div className="space-y-4">
        {filteredEndpoints.map((endpoint, index) => {
          const key = `${endpoint.method}-${endpoint.path}`;
          const isExpanded = expandedEndpoints.has(key);

          return (
            <Card key={index}>
              <Collapsible open={isExpanded} onOpenChange={() => toggleEndpoint(key)}>
                <CollapsibleTrigger className="w-full">
                  <CardHeader className="cursor-pointer hover:bg-muted/50 transition-colors">
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-3">
                        {isExpanded ? (
                          <ChevronDown className="h-5 w-5 text-muted-foreground" />
                        ) : (
                          <ChevronRight className="h-5 w-5 text-muted-foreground" />
                        )}
                        <Badge className={`${getMethodColor(endpoint.method)} text-white`}>
                          {endpoint.method}
                        </Badge>
                        <code className="text-sm font-mono">{endpoint.path}</code>
                      </div>
                      <div className="flex gap-2">
                        {endpoint.tags.map((tag) => (
                          <Badge key={tag} variant="outline">
                            {tag}
                          </Badge>
                        ))}
                      </div>
                    </div>
                    {endpoint.summary && (
                      <CardDescription className="text-left ml-11">
                        {endpoint.summary}
                      </CardDescription>
                    )}
                  </CardHeader>
                </CollapsibleTrigger>

                <CollapsibleContent>
                  <CardContent className="border-t pt-4">
                    {endpoint.description && (
                      <div className="mb-4">
                        <h4 className="font-semibold mb-2">Description</h4>
                        <p className="text-sm text-muted-foreground">{endpoint.description}</p>
                      </div>
                    )}

                    {endpoint.parameters.length > 0 && (
                      <div>
                        <h4 className="font-semibold mb-2">Parameters</h4>
                        <div className="space-y-2">
                          {endpoint.parameters.map((param, idx) => (
                            <div
                              key={idx}
                              className="flex items-start gap-3 p-3 bg-muted rounded-lg"
                            >
                              <div className="flex-1">
                                <div className="flex items-center gap-2 mb-1">
                                  <code className="text-sm font-mono">{param.name}</code>
                                  {param.required && (
                                    <Badge variant="destructive" className="text-xs">
                                      required
                                    </Badge>
                                  )}
                                  <Badge variant="outline" className="text-xs">
                                    {param.location}
                                  </Badge>
                                </div>
                                {param.description && (
                                  <p className="text-sm text-muted-foreground">
                                    {param.description}
                                  </p>
                                )}
                                <div className="text-xs text-muted-foreground mt-1">
                                  Type: <code>{param.schemaType}</code>
                                </div>
                              </div>
                            </div>
                          ))}
                        </div>
                      </div>
                    )}

                    <div className="mt-4">
                      <h4 className="font-semibold mb-2">Response</h4>
                      <div className="p-3 bg-muted rounded-lg">
                        <div className="flex items-center gap-2 mb-2">
                          <Badge className="bg-green-500 text-white">200</Badge>
                          <span className="text-sm">Successful response</span>
                        </div>
                        <pre className="text-xs text-muted-foreground">
                          <code>Content-Type: application/json</code>
                        </pre>
                      </div>
                    </div>
                  </CardContent>
                </CollapsibleContent>
              </Collapsible>
            </Card>
          );
        })}
      </div>
    </div>
  );
}
