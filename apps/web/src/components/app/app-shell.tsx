import {
  Boxes,
  FolderKanban,
  History,
  Inbox,
  LogOut,
  Menu,
  Plus,
  WifiOff,
  X,
} from "lucide-react";
import { useEffect, useState } from "react";
import { NavLink, Outlet, useLocation } from "react-router";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { useAuth } from "@/auth/auth-context";

const rootLinks = [
  { to: "/", label: "Center", icon: Boxes, end: true },
  { to: "/insights", label: "Decision Inbox", icon: Inbox },
];

export function AppShell() {
  const [open, setOpen] = useState(false);
  const [online, setOnline] = useState(() => navigator.onLine);
  const location = useLocation();
  const auth = useAuth();
  const projectId = location.pathname.match(/\/projects\/([^/]+)/)?.[1];
  const projectLinks =
    projectId && projectId !== "new"
      ? [
          {
            to: `/projects/${projectId}`,
            label: "Projet",
            icon: FolderKanban,
            end: true,
          },
          {
            to: `/projects/${projectId}/history`,
            label: "Historique",
            icon: History,
          },
        ]
      : [];

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

  return (
    <div className="min-h-dvh bg-[#f7f8fb] text-slate-950">
      {!online ? (
        <div
          className="fixed inset-x-0 top-0 z-50 flex min-h-10 items-center justify-center gap-2 bg-amber-400 px-4 py-2 text-center text-xs font-semibold text-amber-950"
          role="status"
          aria-live="assertive"
        >
          <WifiOff className="size-4" />
          Hors connexion · vos saisies sont conservées et les actions peuvent
          être réessayées après reconnexion.
        </div>
      ) : null}
      <header className="mobile-safe-header sticky top-0 z-40 flex h-16 items-center justify-between border-b border-white/10 bg-[#11182b] px-4 text-white lg:hidden">
        <Brand />
        <Button
          variant="ghost"
          size="icon"
          className="size-11 text-white hover:bg-white/10 hover:text-white"
          onClick={() => setOpen((value) => !value)}
          aria-label={open ? "Fermer la navigation" : "Ouvrir la navigation"}
        >
          {open ? <X /> : <Menu />}
        </Button>
      </header>
      {open ? (
        <button
          className="fixed inset-0 z-20 bg-slate-950/30 lg:hidden"
          onClick={() => setOpen(false)}
          aria-label="Fermer la navigation"
        />
      ) : null}
      <aside
        className={cn(
          "fixed inset-y-0 left-0 z-30 flex w-64 -translate-x-full flex-col bg-[#11182b] text-slate-300 transition-transform lg:translate-x-0",
          open && "mobile-nav-open translate-x-0 lg:top-0",
        )}
      >
        <div className="hidden h-20 items-center border-b border-white/10 px-6 lg:flex">
          <Brand />
        </div>
        <nav className="flex-1 space-y-7 overflow-y-auto p-4">
          <NavSection
            label="Workspace"
            links={rootLinks}
            close={() => setOpen(false)}
          />
          {projectLinks.length ? (
            <NavSection
              label="Projet ouvert"
              links={projectLinks}
              close={() => setOpen(false)}
            />
          ) : null}
        </nav>
        <div className="border-t border-white/10 p-4">
          <NavLink
            to="/projects/new"
            onClick={() => setOpen(false)}
            className="flex min-h-11 items-center justify-center gap-2 border border-indigo-400/40 bg-indigo-500 px-3 py-2.5 text-sm font-semibold text-white hover:bg-indigo-400"
          >
            <Plus className="size-4" /> Nouveau projet
          </NavLink>
          <div className="mt-4 flex items-center gap-3 px-1">
            <div className="grid size-8 place-items-center bg-emerald-400/15 text-xs font-bold text-emerald-300">
              GM
            </div>
            <div className="min-w-0">
              <p className="truncate text-xs font-semibold text-white">
                Mon workspace
              </p>
              <p className="text-[11px] text-slate-400">Control plane local</p>
            </div>
            {auth.enabled ? (
              <Button
                type="button"
                variant="ghost"
                size="icon"
                className="ml-auto text-slate-300 hover:bg-white/10 hover:text-white"
                aria-label="Se déconnecter"
                onClick={() => void auth.client?.auth.signOut()}
              >
                <LogOut className="size-4" />
              </Button>
            ) : null}
          </div>
        </div>
      </aside>
      <main className="min-h-dvh lg:pl-64">
        <div className="mx-auto max-w-[1500px] px-4 py-7 sm:px-7 lg:px-10 lg:py-10">
          <Outlet />
        </div>
      </main>
    </div>
  );
}

function Brand() {
  return (
    <NavLink to="/" className="flex items-center gap-3 text-white">
      <span className="relative grid size-8 place-items-center border border-indigo-300/30 bg-indigo-400/10">
        <span className="size-2 bg-indigo-300" />
        <span className="absolute -right-1 -top-1 size-2 border border-[#11182b] bg-emerald-400" />
      </span>
      <span>
        <span className="block text-sm font-semibold tracking-[-0.02em]">
          AI Center
        </span>
        <span className="block text-[10px] uppercase tracking-[0.18em] text-slate-400">
          Context control
        </span>
      </span>
    </NavLink>
  );
}

function NavSection({
  label,
  links,
  close,
}: {
  label: string;
  links: Array<{
    to: string;
    label: string;
    icon: typeof Boxes;
    end?: boolean;
  }>;
  close: () => void;
}) {
  return (
    <section>
      <p className="px-3 text-[10px] font-semibold uppercase tracking-[0.2em] text-slate-400">
        {label}
      </p>
      <div className="mt-2 space-y-1">
        {links.map((link) => (
          <NavLink
            key={link.to}
            to={link.to}
            end={link.end}
            onClick={close}
            className={({ isActive }) =>
              cn(
                "flex min-h-11 items-center gap-3 px-3 py-2.5 text-sm transition-colors hover:bg-white/[0.06] hover:text-white",
                isActive && "bg-white/[0.08] text-white",
              )
            }
          >
            <link.icon className="size-4" /> {link.label}
          </NavLink>
        ))}
      </div>
    </section>
  );
}
