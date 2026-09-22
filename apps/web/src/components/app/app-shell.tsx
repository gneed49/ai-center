import { useQuery } from "@tanstack/react-query";
import {
  AlertTriangle,
  Bot,
  FolderKanban,
  FileText,
  History,
  Home,
  Link2,
  LogOut,
  Menu,
  Network,
  Plus,
  Users,
  WifiOff,
  X,
} from "lucide-react";
import { Suspense, useEffect, useState } from "react";
import { NavLink, Outlet, useLocation, useNavigate } from "react-router";

import { api } from "@/api/client";
import { companyApi } from "@/api/company";
import { Button } from "@/components/ui/button";
import { LoadingState } from "@/components/app/page";
import { cn } from "@/lib/utils";
import { useAuth } from "@/auth/auth-context";

const rootLinks = [
  { to: "/", label: "Vue d’ensemble", icon: Home, end: true },
  { to: "/graph", label: "Graphe", icon: Network },
  { to: "/agents", label: "Agents", icon: Bot },
  { to: "/projects", label: "Projets", icon: FolderKanban },
  { to: "/artifacts", label: "Livrables", icon: FileText },
  { to: "/insights", label: "À vérifier", icon: AlertTriangle },
  { to: "/company", label: "Équipe", icon: Users },
  { to: "/settings/ai", label: "Connexions", icon: Link2 },
];
const roleLabels = {
  owner: "Propriétaire",
  editor: "Éditeur",
  viewer: "Lecteur",
};

export function AppShell() {
  const [open, setOpen] = useState(false);
  const [online, setOnline] = useState(() => navigator.onLine);
  const location = useLocation();
  const navigate = useNavigate();
  const auth = useAuth();
  const company = useQuery({
    queryKey: ["company"],
    queryFn: companyApi.overview,
  });
  const workspaces = useQuery({
    queryKey: ["workspaces"],
    queryFn: api.workspaces,
    enabled: auth.enabled,
  });
  const routeProject = location.pathname.match(/\/projects\/([^/]+)/)?.[1];
  const queryProject = new URLSearchParams(location.search).get("project");
  const projectId =
    routeProject && routeProject !== "new" ? routeProject : queryProject;
  const isCompanyScope =
    company.data?.company_scope?.project_public_id === projectId;
  const project = company.data?.projects.find(
    (item) => item.public_id === projectId,
  );
  const projectLinks =
    projectId && !isCompanyScope
      ? [
          {
            to: `/projects/${projectId}`,
            label: "Vue du projet",
            icon: FolderKanban,
            end: true,
          },
          {
            to: `/agents?project=${projectId}`,
            label: "Agents du projet",
            icon: Bot,
            exactSearch: true,
          },
          {
            to: `/graph?project=${projectId}`,
            label: "Graphe du projet",
            icon: Network,
            exactSearch: true,
          },
          {
            to: `/projects/${projectId}/artifacts`,
            label: "Livrables du projet",
            icon: FileText,
          },
          {
            to: `/projects/${projectId}/deliverables`,
            label: "Plans et preuves",
            icon: FileText,
          },
          {
            to: `/projects/${projectId}/code`,
            label: "Preuves de code",
            icon: FileText,
          },
          {
            to: `/projects/${projectId}/history`,
            label: "Historique",
            icon: History,
          },
        ]
      : [];
  const identityLabel =
    auth.session?.user.email ??
    (auth.enabled ? "Compte connecté" : "Session locale");
  const initials = identityLabel.slice(0, 2).toUpperCase();
  const workspaceName =
    company.data?.workspace.name ??
    workspaces.data?.find((item) => item.public_id === auth.workspaceId)?.name;

  useEffect(() => {
    const markOnline = () => setOnline(true);
    const markOffline = () => setOnline(false);
    window.addEventListener("online", markOnline);
    window.addEventListener("offline", markOffline);
    return () => {
      window.removeEventListener("online", markOnline);
      window.removeEventListener("offline", markOffline);
    };
  }, []);

  useEffect(() => {
    if (!open) return;
    const escape = (event: KeyboardEvent) => {
      if (event.key === "Escape") setOpen(false);
    };
    window.addEventListener("keydown", escape);
    return () => window.removeEventListener("keydown", escape);
  }, [open]);

  return (
    <div className="min-h-dvh bg-background text-foreground">
      <a
        href="#main-content"
        className="sr-only focus:not-sr-only focus:fixed focus:left-4 focus:top-4 focus:z-[100] focus:rounded-md focus:bg-primary focus:px-4 focus:py-3 focus:text-white"
      >
        Aller au contenu
      </a>
      {!online ? (
        <div
          className="fixed inset-x-0 top-0 z-50 flex min-h-10 items-center justify-center gap-2 bg-amber-100 px-4 py-2 text-center text-sm text-amber-950"
          role="status"
          aria-live="assertive"
        >
          <WifiOff className="size-4 shrink-0" /> Hors connexion. Restez sur
          cette page pour conserver votre saisie, puis réessayez après
          reconnexion.
        </div>
      ) : null}
      <header className="mobile-safe-header sticky top-0 z-40 flex h-16 items-center justify-between border-b bg-white px-4 lg:hidden">
        <Brand />
        <Button
          variant="ghost"
          size="icon"
          className="size-11"
          onClick={() => setOpen((value) => !value)}
          aria-expanded={open}
          aria-controls="app-sidebar"
          aria-label={open ? "Fermer la navigation" : "Ouvrir la navigation"}
        >
          {open ? <X /> : <Menu />}
        </Button>
      </header>
      {open ? (
        <button
          className="fixed inset-0 z-20 bg-slate-950/20 lg:hidden"
          onClick={() => setOpen(false)}
          aria-label="Fermer le menu"
        />
      ) : null}
      <aside
        id="app-sidebar"
        className={cn(
          "fixed inset-y-0 left-0 z-30 flex w-60 -translate-x-full flex-col border-r border-sidebar-border bg-sidebar text-sidebar-foreground transition-transform lg:translate-x-0",
          open ? "mobile-nav-open translate-x-0 lg:top-0" : "hidden lg:flex",
        )}
      >
        <div className="hidden h-24 items-center px-6 lg:flex">
          <Brand />
        </div>
        <div className="px-4 pb-5 pt-5 lg:pt-2">
          <p
            id="company-switcher-label"
            className="mb-2 block text-xs font-medium"
          >
            Entreprise
          </p>
          {auth.enabled && workspaces.data?.length ? (
            <select
              id="company-switcher"
              aria-labelledby="company-switcher-label"
              className="min-h-10 w-full rounded-md border bg-white px-3 text-sm text-foreground"
              value={auth.workspaceId ?? ""}
              onChange={(event) => {
                if (event.target.value === auth.workspaceId) return;
                auth.selectWorkspace(event.target.value);
                setOpen(false);
                navigate("/");
              }}
            >
              {workspaces.data.map((item) => (
                <option key={item.public_id} value={item.public_id}>
                  {item.name}
                </option>
              ))}
            </select>
          ) : (
            <div
              id="company-switcher"
              className="min-h-10 rounded-md border bg-white px-3 py-2 text-sm text-foreground"
            >
              {workspaceName ??
                (company.isPending ? "Chargement…" : "Entreprise indisponible")}
            </div>
          )}
          {company.isError ? (
            <button
              className="mt-2 text-xs text-primary underline underline-offset-2"
              onClick={() => void company.refetch()}
            >
              Actualiser l’entreprise
            </button>
          ) : null}
        </div>
        <nav
          aria-label="Navigation principale"
          className="flex-1 space-y-6 overflow-y-auto px-2 py-1"
        >
          <NavSection links={rootLinks} close={() => setOpen(false)} />
          {projectLinks.length ? (
            <NavSection
              label={project?.name ?? "Projet ouvert"}
              links={projectLinks}
              close={() => setOpen(false)}
            />
          ) : null}
        </nav>
        <div className="p-4">
          <Button
            asChild
            variant="outline"
            className="mb-5 w-full justify-start bg-white"
          >
            <NavLink to="/projects/new" onClick={() => setOpen(false)}>
              <Plus /> Nouveau projet
            </NavLink>
          </Button>
          <div className="flex items-center gap-3 border-t border-sidebar-border pt-4">
            <span
              aria-hidden
              className="grid size-9 shrink-0 place-items-center rounded-full bg-primary text-xs font-semibold text-white"
            >
              {initials}
            </span>
            <div className="min-w-0 flex-1">
              <p
                className="truncate text-xs font-medium text-foreground"
                title={identityLabel}
              >
                {identityLabel}
              </p>
              <p className="mt-1 text-xs text-muted-foreground">
                {company.data
                  ? roleLabels[company.data.workspace.role]
                  : auth.enabled
                    ? "Compte connecté"
                    : "Accès local"}
              </p>
            </div>
            {auth.enabled ? (
              <Button
                type="button"
                variant="ghost"
                size="icon"
                className="size-8 shrink-0"
                aria-label="Se déconnecter"
                onClick={() => void auth.client?.auth.signOut()}
              >
                <LogOut className="size-4" />
              </Button>
            ) : null}
          </div>
        </div>
      </aside>
      <main id="main-content" className="min-h-dvh lg:pl-60" tabIndex={-1}>
        <div className="mx-auto max-w-[1720px] px-4 py-6 sm:px-7 lg:px-8 lg:py-8">
          <Suspense fallback={<LoadingState label="Ouverture de la page…" />}>
            <Outlet />
          </Suspense>
        </div>
      </main>
    </div>
  );
}

function Brand() {
  return (
    <NavLink to="/" className="flex items-center gap-3 text-foreground">
      <Network aria-hidden className="size-8 text-primary" strokeWidth={2.5} />
      <span className="text-xl font-semibold tracking-tight">AI Center</span>
    </NavLink>
  );
}

function NavSection({
  label,
  links,
  close,
}: {
  label?: string;
  links: Array<{
    to: string;
    label: string;
    icon: typeof Home;
    end?: boolean;
    exactSearch?: boolean;
  }>;
  close: () => void;
}) {
  const location = useLocation();
  return (
    <section>
      {label ? (
        <p
          className="truncate px-4 pb-2 text-xs font-medium text-muted-foreground"
          title={label}
        >
          {label}
        </p>
      ) : null}
      <div className="space-y-1">
        {links.map((link) => (
          <NavLink
            key={link.to}
            to={link.to}
            end={link.end}
            onClick={close}
            className={({ isActive }) =>
              cn(
                "flex min-h-11 items-center gap-3 rounded-md border-l-[3px] border-transparent px-4 py-2.5 text-sm transition-colors hover:bg-sidebar-accent/60",
                isActive &&
                  (!link.exactSearch ||
                    `${location.pathname}${location.search}` === link.to) &&
                  "border-primary bg-sidebar-accent font-medium text-sidebar-accent-foreground",
              )
            }
          >
            <link.icon aria-hidden className="size-[18px] shrink-0" />
            {link.label}
          </NavLink>
        ))}
      </div>
    </section>
  );
}
