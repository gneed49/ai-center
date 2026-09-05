import { Route, Routes, useLocation } from "react-router";

import { AppShell } from "@/components/app/app-shell";
import { CenterPage } from "@/pages/center-page";
import { DeliverableDetailPage } from "@/pages/deliverable-detail-page";
import { DeliverablesPage } from "@/pages/deliverables-page";
import { HandoffPage } from "@/pages/handoff-page";
import { HistoryPage } from "@/pages/history-page";
import { InsightDetailPage } from "@/pages/insight-detail-page";
import { InsightsPage } from "@/pages/insights-page";
import { NewProjectPage } from "@/pages/new-project-page";
import { ProjectPage } from "@/pages/project-page";
import { SessionPage } from "@/pages/session-page";
import { NotFoundState } from "@/components/app/page";
import { useAuth } from "@/auth/auth-context";
import {
  AuthCallbackPage,
  LoginPage,
  WorkspacePicker,
} from "@/auth/auth-pages";

export default function App() {
  const auth = useAuth();
  const location = useLocation();
  if (auth.enabled && location.pathname === "/auth/callback") {
    return <AuthCallbackPage />;
  }
  if (auth.enabled && auth.loading) {
    return (
      <main
        className="grid min-h-dvh place-items-center bg-[#11182b] text-sm text-slate-300"
        role="status"
      >
        Restauration de la session sécurisée…
      </main>
    );
  }
  if (auth.enabled && !auth.session) return <LoginPage />;
  if (auth.enabled && !auth.workspaceId) {
    return <WorkspacePicker />;
  }
  return (
    <Routes>
      <Route element={<AppShell />}>
        <Route index element={<CenterPage />} />
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
        <Route path="insights" element={<InsightsPage />} />
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
