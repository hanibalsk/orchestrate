import { useState, useEffect } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../components/ui/card';
import { Button } from '../components/ui/button';
import { Badge } from '../components/ui/badge';
import { FileText, Calendar, CheckCircle, XCircle, AlertCircle, ArrowLeft } from 'lucide-react';

interface Adr {
  number: number;
  title: string;
  status: 'proposed' | 'accepted' | 'deprecated' | 'superseded' | 'rejected';
  date: string;
  context: string;
  decision: string;
  consequences: AdrConsequence[];
  relatedAdrs: number[];
  supersededBy?: number;
  tags: string[];
}

interface AdrConsequence {
  positive: boolean;
  description: string;
}

export function AdrBrowser() {
  const { number } = useParams<{ number: string }>();
  const navigate = useNavigate();
  const [adrs, setAdrs] = useState<Adr[]>([]);
  const [selectedAdr, setSelectedAdr] = useState<Adr | null>(null);
  const [loading, setLoading] = useState(true);
  const [filter, setFilter] = useState<string>('all');

  useEffect(() => {
    loadAdrs();
  }, []);

  useEffect(() => {
    if (number && adrs.length > 0) {
      const adr = adrs.find((a) => a.number === parseInt(number));
      setSelectedAdr(adr || null);
    }
  }, [number, adrs]);

  const loadAdrs = async () => {
    try {
      // Mock data - would call API
      const mockAdrs: Adr[] = [
        {
          number: 1,
          title: 'Use SQLite for Agent State Storage',
          status: 'accepted',
          date: '2024-01-15T00:00:00Z',
          context: 'We need a reliable way to persist agent state across restarts and crashes.',
          decision: 'We will use SQLite as the primary database for agent state storage.',
          consequences: [
            {
              positive: true,
              description: 'Simple deployment with no external database required',
            },
            {
              positive: true,
              description: 'Good performance for single-node deployments',
            },
            {
              positive: false,
              description: 'Limited to single-node architecture',
            },
          ],
          relatedAdrs: [],
          tags: ['database', 'architecture'],
        },
        {
          number: 2,
          title: 'Use TypeScript + React for Web UI',
          status: 'accepted',
          date: '2024-01-20T00:00:00Z',
          context: 'We need a modern, maintainable web interface for managing agents.',
          decision: 'Use TypeScript with React and Vite for the web frontend.',
          consequences: [
            {
              positive: true,
              description: 'Type safety reduces runtime errors',
            },
            {
              positive: true,
              description: 'Large ecosystem of libraries and tools',
            },
            {
              positive: false,
              description: 'Steeper learning curve for contributors',
            },
          ],
          relatedAdrs: [],
          tags: ['frontend', 'architecture'],
        },
      ];
      setAdrs(mockAdrs);
    } catch (error) {
      console.error('Failed to load ADRs:', error);
    } finally {
      setLoading(false);
    }
  };

  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'accepted':
        return <CheckCircle className="h-5 w-5 text-green-500" />;
      case 'proposed':
        return <AlertCircle className="h-5 w-5 text-yellow-500" />;
      case 'deprecated':
      case 'superseded':
      case 'rejected':
        return <XCircle className="h-5 w-5 text-red-500" />;
      default:
        return <FileText className="h-5 w-5" />;
    }
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'accepted':
        return 'bg-green-500/10 text-green-700 border-green-500/20';
      case 'proposed':
        return 'bg-yellow-500/10 text-yellow-700 border-yellow-500/20';
      case 'deprecated':
      case 'superseded':
        return 'bg-orange-500/10 text-orange-700 border-orange-500/20';
      case 'rejected':
        return 'bg-red-500/10 text-red-700 border-red-500/20';
      default:
        return 'bg-gray-500/10 text-gray-700 border-gray-500/20';
    }
  };

  const filteredAdrs = filter === 'all'
    ? adrs
    : adrs.filter((adr) => adr.status === filter);

  if (loading) {
    return (
      <div className="space-y-6">
        <h1 className="text-3xl font-bold">Architecture Decision Records</h1>
        <div className="text-muted-foreground">Loading...</div>
      </div>
    );
  }

  if (selectedAdr) {
    return (
      <div className="space-y-6">
        <div className="flex items-center gap-4">
          <Button variant="ghost" onClick={() => {
            setSelectedAdr(null);
            navigate('/docs/adr');
          }}>
            <ArrowLeft className="h-4 w-4 mr-2" />
            Back to List
          </Button>
        </div>

        <Card>
          <CardHeader>
            <div className="flex items-start justify-between">
              <div className="space-y-2">
                <div className="flex items-center gap-3">
                  <Badge variant="outline" className="text-lg px-3 py-1">
                    ADR-{selectedAdr.number.toString().padStart(4, '0')}
                  </Badge>
                  <Badge className={getStatusColor(selectedAdr.status)}>
                    {getStatusIcon(selectedAdr.status)}
                    <span className="ml-2">{selectedAdr.status}</span>
                  </Badge>
                </div>
                <CardTitle className="text-3xl">{selectedAdr.title}</CardTitle>
                <div className="flex items-center gap-2 text-sm text-muted-foreground">
                  <Calendar className="h-4 w-4" />
                  {new Date(selectedAdr.date).toLocaleDateString()}
                </div>
              </div>
            </div>
            {selectedAdr.tags.length > 0 && (
              <div className="flex gap-2 mt-4">
                {selectedAdr.tags.map((tag) => (
                  <Badge key={tag} variant="secondary">
                    {tag}
                  </Badge>
                ))}
              </div>
            )}
          </CardHeader>
        </Card>

        {selectedAdr.supersededBy && (
          <Card className="border-orange-500/50 bg-orange-500/5">
            <CardContent className="pt-6">
              <div className="flex items-center gap-2">
                <AlertCircle className="h-5 w-5 text-orange-500" />
                <span className="text-sm">
                  This decision has been superseded by{' '}
                  <Button
                    variant="link"
                    className="p-0 h-auto text-orange-700"
                    onClick={() => navigate(`/docs/adr/${selectedAdr.supersededBy}`)}
                  >
                    ADR-{selectedAdr.supersededBy.toString().padStart(4, '0')}
                  </Button>
                </span>
              </div>
            </CardContent>
          </Card>
        )}

        <Card>
          <CardHeader>
            <CardTitle>Context</CardTitle>
          </CardHeader>
          <CardContent>
            <p className="whitespace-pre-wrap">{selectedAdr.context}</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Decision</CardTitle>
          </CardHeader>
          <CardContent>
            <p className="whitespace-pre-wrap">{selectedAdr.decision}</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Consequences</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            {selectedAdr.consequences.filter((c) => c.positive).length > 0 && (
              <div>
                <h4 className="font-semibold text-green-700 mb-2">Positive</h4>
                <ul className="space-y-2">
                  {selectedAdr.consequences
                    .filter((c) => c.positive)
                    .map((c, idx) => (
                      <li key={idx} className="flex gap-2">
                        <CheckCircle className="h-5 w-5 text-green-500 flex-shrink-0 mt-0.5" />
                        <span>{c.description}</span>
                      </li>
                    ))}
                </ul>
              </div>
            )}

            {selectedAdr.consequences.filter((c) => !c.positive).length > 0 && (
              <div>
                <h4 className="font-semibold text-red-700 mb-2">Negative</h4>
                <ul className="space-y-2">
                  {selectedAdr.consequences
                    .filter((c) => !c.positive)
                    .map((c, idx) => (
                      <li key={idx} className="flex gap-2">
                        <XCircle className="h-5 w-5 text-red-500 flex-shrink-0 mt-0.5" />
                        <span>{c.description}</span>
                      </li>
                    ))}
                </ul>
              </div>
            )}
          </CardContent>
        </Card>

        {selectedAdr.relatedAdrs.length > 0 && (
          <Card>
            <CardHeader>
              <CardTitle>Related ADRs</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="flex flex-wrap gap-2">
                {selectedAdr.relatedAdrs.map((num) => (
                  <Button
                    key={num}
                    variant="outline"
                    onClick={() => navigate(`/docs/adr/${num}`)}
                  >
                    ADR-{num.toString().padStart(4, '0')}
                  </Button>
                ))}
              </div>
            </CardContent>
          </Card>
        )}
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold">Architecture Decision Records</h1>
          <p className="text-muted-foreground">
            Document and track architectural decisions
          </p>
        </div>
        <Button onClick={() => navigate('/docs/adr/new')}>
          Create ADR
        </Button>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Filter by Status</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="flex flex-wrap gap-2">
            {['all', 'proposed', 'accepted', 'deprecated', 'rejected'].map((status) => (
              <Badge
                key={status}
                variant={filter === status ? 'default' : 'outline'}
                className="cursor-pointer"
                onClick={() => setFilter(status)}
              >
                {status}
              </Badge>
            ))}
          </div>
        </CardContent>
      </Card>

      <div className="grid grid-cols-1 gap-4">
        {filteredAdrs.map((adr) => (
          <Card
            key={adr.number}
            className="hover:shadow-lg transition-shadow cursor-pointer"
            onClick={() => navigate(`/docs/adr/${adr.number}`)}
          >
            <CardHeader>
              <div className="flex items-start justify-between">
                <div className="space-y-2 flex-1">
                  <div className="flex items-center gap-3">
                    <Badge variant="outline">
                      ADR-{adr.number.toString().padStart(4, '0')}
                    </Badge>
                    <Badge className={getStatusColor(adr.status)}>
                      {getStatusIcon(adr.status)}
                      <span className="ml-2">{adr.status}</span>
                    </Badge>
                  </div>
                  <CardTitle>{adr.title}</CardTitle>
                  <CardDescription>
                    {new Date(adr.date).toLocaleDateString()}
                  </CardDescription>
                </div>
              </div>
              {adr.tags.length > 0 && (
                <div className="flex gap-2 mt-2">
                  {adr.tags.map((tag) => (
                    <Badge key={tag} variant="secondary">
                      {tag}
                    </Badge>
                  ))}
                </div>
              )}
            </CardHeader>
          </Card>
        ))}
      </div>

      {filteredAdrs.length === 0 && (
        <Card>
          <CardContent className="py-12 text-center">
            <FileText className="h-12 w-12 mx-auto mb-4 text-muted-foreground" />
            <h3 className="font-semibold mb-2">No ADRs found</h3>
            <p className="text-sm text-muted-foreground mb-4">
              Create your first ADR to document architectural decisions
            </p>
            <Button onClick={() => navigate('/docs/adr/new')}>
              Create ADR
            </Button>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
