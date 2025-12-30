import { BrowserRouter, Routes, Route } from 'react-router-dom';
import { Navbar } from './components/layout/Navbar';
import { Dashboard } from './pages/Dashboard';
import { AgentList } from './pages/AgentList';
import { AgentDetail } from './pages/AgentDetail';
import { PipelineList } from './pages/PipelineList';
import { PipelineDetail } from './pages/PipelineDetail';
import { PipelineRunDetail } from './pages/PipelineRunDetail';
import { PipelineNew } from './pages/PipelineNew';
import { ScheduleList } from './pages/ScheduleList';
import { DocsList } from './pages/DocsList';
import { ApiDocsViewer } from './pages/ApiDocsViewer';
import { AdrBrowser } from './pages/AdrBrowser';

function App() {
  return (
    <BrowserRouter>
      <div className="min-h-screen bg-background">
        <Navbar />
        <main className="container mx-auto max-w-7xl px-4 py-8">
          <Routes>
            <Route path="/" element={<Dashboard />} />
            <Route path="/agents" element={<AgentList />} />
            <Route path="/agents/:id" element={<AgentDetail />} />
            <Route path="/pipelines" element={<PipelineList />} />
            <Route path="/pipelines/new" element={<PipelineNew />} />
            <Route path="/pipelines/:name" element={<PipelineDetail />} />
            <Route path="/pipelines/:name/runs/:runId" element={<PipelineRunDetail />} />
            <Route path="/schedules" element={<ScheduleList />} />
            <Route path="/docs" element={<DocsList />} />
            <Route path="/docs/api" element={<ApiDocsViewer />} />
            <Route path="/docs/adr" element={<AdrBrowser />} />
            <Route path="/docs/adr/:number" element={<AdrBrowser />} />
          </Routes>
        </main>
      </div>
    </BrowserRouter>
  );
}

export default App;
