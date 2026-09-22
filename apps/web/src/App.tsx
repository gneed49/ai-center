import { lazy, Suspense } from "react";
import { Route, Routes, useLocation } from "react-router";

import { AppShell } from "@/components/app/app-shell";
import { LoadingState, NotFoundState } from "@/components/app/page";
import { useAuth } from "@/auth/auth-context";
import {
  AuthCallbackPage,
  LoginPage,
  WorkspacePicker,
} from "@/auth/auth-pages";

const JoinCompanyPage = lazy(() =>
  import("@/pages/join-company-page").then((module) => ({
    default: module.JoinCompanyPage,
  })),
);
const CenterPage = lazy(() =>
  import("@/pages/center-page").then((module) => ({
    default: module.CenterPage,
  })),
);
const CompanyPage = lazy(() =>
  import("@/pages/company-page").then((module) => ({
    default: module.CompanyPage,
  })),
);
const GraphSourcePage = lazy(() =>
  import("@/pages/graph-source-page").then((module) => ({
    default: module.GraphSourcePage,
  })),
);
const GraphPage = lazy(() =>
  import("@/pages/graph-page").then((module) => ({
    default: module.GraphPage,
  })),
);
const AgentsPage = lazy(() =>
  import("@/pages/agents-page").then((module) => ({
    default: module.AgentsPage,
  })),
);
const ArtifactsPage = lazy(() =>
  import("@/pages/artifacts-page").then((module) => ({
    default: module.ArtifactsPage,
  })),
);
const ArtifactPage = lazy(() =>
  import("@/pages/artifact-page").then((module) => ({
    default: module.ArtifactPage,
  })),
);
const ArtifactDestinationsPage = lazy(() =>
  import("@/pages/artifact-destinations-page").then((module) => ({
    default: module.ArtifactDestinationsPage,
  })),
);
const CodeEvidencePage = lazy(() =>
  import("@/pages/code-evidence-page").then((module) => ({
    default: module.CodeEvidencePage,
  })),
);
const WorkToolsPage = lazy(() =>
  import("@/pages/work-tools-page").then((module) => ({
    default: module.WorkToolsPage,
  })),
);
const AiSettingsPage = lazy(() =>
  import("@/pages/ai-settings-page").then((module) => ({
    default: module.AiSettingsPage,
  })),
);
const DeliverableDetailPage = lazy(() =>
  import("@/pages/deliverable-detail-page").then((module) => ({
    default: module.DeliverableDetailPage,
  })),
);
const DeliverablesPage = lazy(() =>
  import("@/pages/deliverables-page").then((module) => ({
    default: module.DeliverablesPage,
  })),
);
const HandoffPage = lazy(() =>
  import("@/pages/handoff-page").then((module) => ({
    default: module.HandoffPage,
  })),
);
const HistoryPage = lazy(() =>
  import("@/pages/history-page").then((module) => ({
    default: module.HistoryPage,
  })),
);
const InsightDetailPage = lazy(() =>
  import("@/pages/insight-detail-page").then((module) => ({
    default: module.InsightDetailPage,
  })),
);
const InsightsPage = lazy(() =>
  import("@/pages/insights-page").then((module) => ({
    default: module.InsightsPage,
  })),
);
const NewProjectPage = lazy(() =>
  import("@/pages/new-project-page").then((module) => ({
    default: module.NewProjectPage,
  })),
);
const ProjectPage = lazy(() =>
  import("@/pages/project-page").then((module) => ({
    default: module.ProjectPage,
  })),
);
const SessionPage = lazy(() =>
  import("@/pages/session-page").then((module) => ({
    default: module.SessionPage,
  })),
);

export default function App() {
  const auth = useAuth();
  const location = useLocation();
  if (/^\/join\/[^/]+\/?$/.test(location.pathname)) {
    return (
      <Suspense fallback={<LoadingState label="Chargement de l’invitation…" />}>
        <Routes>
          <Route path="join/:invitationId" element={<JoinCompanyPage />} />
        </Routes>
      </Suspense>
    );
  }
  if (auth.enabled && location.pathname === "/auth/callback")
    return <AuthCallbackPage />;
  if (auth.enabled && auth.loading)
    return (
      <main
        className="grid min-h-dvh place-items-center bg-background text-sm text-muted-foreground"
        role="status"
      >
        Restauration de votre connexion…
      </main>
    );
  if (auth.enabled && !auth.session) return <LoginPage />;
  if (auth.enabled && !auth.workspaceId) return <WorkspacePicker />;
  return (
    <Routes>
      <Route element={<AppShell />}>
        <Route index element={<CenterPage />} />
        <Route path="projects" element={<CenterPage />} />
        <Route path="company" element={<CompanyPage />} />
        <Route path="graph" element={<GraphPage />} />
        <Route
          path="projects/:projectId/sources/:kind/:sourceId"
          element={<GraphSourcePage />}
        />
        <Route path="agents" element={<AgentsPage />} />
        <Route path="artifacts" element={<ArtifactsPage />} />
        <Route path="artifacts/:artifactId" element={<ArtifactPage />} />
        <Route
          path="projects/:projectId/artifacts"
          element={<ArtifactsPage />}
        />
        <Route
          path="settings/destinations"
          element={<ArtifactDestinationsPage />}
        />
        <Route path="projects/new" element={<NewProjectPage />} />
        <Route path="projects/:projectId" element={<ProjectPage />} />
        <Route
          path="projects/:projectId/sessions/:sessionId"
          element={<SessionPage />}
        />
        <Route
          path="projects/:projectId/deliverables"
          element={<DeliverablesPage />}
        />
        <Route
          path="projects/:projectId/deliverables/:deliverableId"
          element={<DeliverableDetailPage />}
        />
        <Route path="projects/:projectId/handoff" element={<HandoffPage />} />
        <Route path="projects/:projectId/history" element={<HistoryPage />} />
        <Route path="projects/:projectId/code" element={<CodeEvidencePage />} />
        <Route path="insights" element={<InsightsPage />} />
        <Route path="settings/ai" element={<AiSettingsPage />} />
        <Route path="settings/tools" element={<WorkToolsPage />} />
        <Route
          path="projects/:projectId/insights/:insightId"
          element={<InsightDetailPage />}
        />
        <Route path="insights/:insightId" element={<InsightDetailPage />} />
        <Route path="*" element={<NotFoundState />} />
      </Route>
    </Routes>
  );
}
