import { Navigate, Route, Routes } from "react-router";

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

export default function App() {
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
        <Route path="insights/:insightId" element={<InsightDetailPage />} />
        <Route path="*" element={<Navigate to="/" replace />} />
      </Route>
    </Routes>
  );
}
