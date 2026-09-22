import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useRef, useState } from "react";
import type { FormEvent } from "react";
import { Copy, UserPlus, Users } from "lucide-react";
import { useAuth } from "@/auth/auth-context";
import { companyApi } from "@/api/company";
import { createIdempotencyKey } from "@/api/client";
import { teamApi } from "@/api/team";
import type {
  TeamMember,
  InvitationRole,
  TeamInvitation,
} from "@/api/team-types";
import { invitationLink, prepareTeamInvitation } from "@/lib/team-invitation";
import { formatDate } from "@/lib/format";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ErrorState, LoadingState } from "@/components/app/page";

const roles = {
  owner: "Propriétaire",
  editor: "Collaborateur",
  viewer: "Lecteur",
};
const memberStates = {
  pending: "En attente",
  accepted: "Actif",
  revoked: "Accès retiré",
};
const invitationStates = {
  pending: "En attente",
  accepted: "Acceptée",
  revoked: "Révoquée",
  expired: "Expirée",
};
export function TeamAccess({ role }: { role: TeamMember["role"] }) {
  const auth = useAuth();
  const cache = useQueryClient();
  const members = useQuery({
    queryKey: ["team-members"],
    queryFn: teamApi.members,
  });
  const command = useRef<{ fingerprint: string; key: string } | null>(null);
  const change = useMutation({
    mutationFn: ({
      member,
      role,
      status,
    }: {
      member: TeamMember;
      role?: TeamMember["role"];
      status?: TeamMember["invitation_status"];
    }) => {
      const input = { role, invitation_status: status };
      const fingerprint = JSON.stringify([member.public_id, input]);
      if (command.current?.fingerprint !== fingerprint)
        command.current = { fingerprint, key: createIdempotencyKey() };
      return companyApi.updateMember(
        member.public_id,
        input,
        command.current.key,
      );
    },
    onSuccess: async () => {
      command.current = null;
      await Promise.all([
        cache.invalidateQueries({ queryKey: ["team-members"] }),
        cache.invalidateQueries({ queryKey: ["company"] }),
        cache.invalidateQueries({ queryKey: ["workspaces"] }),
      ]);
    },
  });
  const self = members.data?.find(
    (member) => member.actor_id === auth.session?.user.id,
  );
  const owners =
    members.data?.filter(
      (member) =>
        member.role === "owner" && member.invitation_status === "accepted",
    ).length ?? 0;
  return (
    <section id="team" className="space-y-8 border-t pt-8">
      <div>
        <div className="flex items-center gap-3">
          <Users className="size-5 text-primary" />
          <h2 className="text-xl font-semibold">Votre équipe</h2>
        </div>
        <p className="mt-2 text-sm text-muted-foreground">
          Les collaborateurs créent et modifient le travail. Les lecteurs le
          consultent. Les propriétaires gèrent les accès à tous les projets de
          cette entreprise.
        </p>
      </div>
      <ProfileName
        key={self?.public_id ?? "self"}
        initialName={self?.display_name ?? ""}
      />
      {members.isPending ? (
        <LoadingState label="Chargement de l’équipe…" />
      ) : members.isError ? (
        <ErrorState
          error={members.error}
          retry={() => void members.refetch()}
        />
      ) : (
        <ul className="divide-y rounded-lg border">
          {members.data.map((member) => {
            const isSelf = member.actor_id === auth.session?.user.id;
            const name =
              member.display_name ||
              (isSelf
                ? (auth.session?.user.email ?? "Vous")
                : "Membre sans nom affiché");
            const lastOwner =
              member.role === "owner" &&
              member.invitation_status === "accepted" &&
              owners === 1;
            return (
              <li
                key={member.public_id}
                className="flex flex-wrap items-center gap-4 p-5"
              >
                <div className="min-w-0 flex-1">
                  <p className="break-words text-sm font-medium">
                    {name}
                    {isSelf ? (
                      <span className="ml-2 text-xs text-muted-foreground">
                        vous
                      </span>
                    ) : null}
                  </p>
                  <p className="mt-1 text-xs text-muted-foreground">
                    {memberStates[member.invitation_status]}
                  </p>
                </div>
                {role === "owner" ? (
                  <>
                    <select
                      aria-label={`Rôle de ${name}`}
                      value={member.role}
                      disabled={
                        change.isPending ||
                        lastOwner ||
                        member.invitation_status !== "accepted"
                      }
                      className="min-h-10 rounded-md border bg-white px-3 text-sm"
                      onChange={(event) =>
                        change.mutate({
                          member,
                          role: event.target.value as TeamMember["role"],
                        })
                      }
                    >
                      {Object.entries(roles).map(([key, value]) => (
                        <option key={key} value={key}>
                          {value}
                        </option>
                      ))}
                    </select>
                    <Button
                      variant="ghost"
                      size="sm"
                      aria-label={`Retirer l’accès de ${name}`}
                      disabled={
                        change.isPending ||
                        lastOwner ||
                        member.invitation_status !== "accepted"
                      }
                      onClick={() =>
                        change.mutate({ member, status: "revoked" })
                      }
                    >
                      Retirer l’accès
                    </Button>
                  </>
                ) : (
                  <span className="text-sm text-muted-foreground">
                    {roles[member.role]}
                  </span>
                )}
              </li>
            );
          })}
        </ul>
      )}
      {change.error ? (
        <ErrorState
          title="La modification d’accès n’a pas pu être confirmée"
          error={change.error}
        />
      ) : null}
      <p className="text-xs text-muted-foreground">
        L’entreprise conserve au moins un propriétaire actif. Le nom affiché
        sert à reconnaître les membres et ne détermine pas leurs droits.
      </p>
      {role === "owner" ? <TeamInvitations /> : null}
    </section>
  );
}
function ProfileName({ initialName }: { initialName: string }) {
  const [name, setName] = useState(initialName);
  const [saved, setSaved] = useState(false);
  const previous = useRef<{ name: string; key: string } | null>(null);
  const cache = useQueryClient();
  const save = useMutation({
    mutationFn: () => {
      const value = name.trim();
      if (previous.current?.name !== value)
        previous.current = { name: value, key: createIdempotencyKey() };
      return teamApi.profile(value, previous.current.key);
    },
    onSuccess: () => {
      previous.current = null;
      setSaved(true);
      void cache.invalidateQueries({ queryKey: ["team-members"] });
    },
  });
  return (
    <form
      className="space-y-3 rounded-lg bg-muted/40 p-5"
      onSubmit={(event) => {
        event.preventDefault();
        setSaved(false);
        if (name.trim()) save.mutate();
      }}
    >
      <label htmlFor="team-display-name" className="block text-sm font-medium">
        Votre nom dans l’équipe
      </label>
      <div className="flex flex-col gap-3 sm:flex-row">
        <Input
          id="team-display-name"
          className="sm:max-w-md"
          value={name}
          maxLength={120}
          autoComplete="name"
          required
          disabled={save.isPending}
          onChange={(event) => {
            setName(event.target.value);
            setSaved(false);
          }}
        />
        <Button
          type="submit"
          variant="outline"
          disabled={save.isPending || !name.trim()}
        >
          {save.isPending ? "Enregistrement…" : "Enregistrer mon nom"}
        </Button>
      </div>
      {save.error ? <ErrorState error={save.error} /> : null}
      {saved ? (
        <p role="status" className="text-sm text-emerald-800">
          Votre nom est enregistré.
        </p>
      ) : null}
    </form>
  );
}
function TeamInvitations() {
  const cache = useQueryClient();
  const list = useQuery({
    queryKey: ["team-invitations"],
    queryFn: teamApi.invitations,
  });
  const [role, setRole] = useState<InvitationRole>("editor");
  const [label, setLabel] = useState("");
  const [days, setDays] = useState(7);
  const [link, setLink] = useState<{ id: string; url: string } | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const previous = useRef<{
    fingerprint: string;
    command: Promise<Awaited<ReturnType<typeof prepareTeamInvitation>>>;
  } | null>(null);
  const revokeKey = useRef<{ id: string; key: string } | null>(null);
  const create = useMutation({
    mutationFn: async () => {
      const input = { role, label: label.trim(), expires_in_days: days };
      const fingerprint = JSON.stringify(input);
      if (previous.current?.fingerprint !== fingerprint)
        previous.current = {
          fingerprint,
          command: prepareTeamInvitation(input),
        };
      const command = await previous.current.command;
      return teamApi.createInvitation(command.payload, command.key);
    },
    onSuccess: async (invitation) => {
      const command = await previous.current!.command;
      setLink({
        id: invitation.public_id,
        url: invitationLink(
          window.location.origin,
          invitation.public_id,
          command.token,
        ),
      });
      setNotice(null);
      void cache.invalidateQueries({ queryKey: ["team-invitations"] });
    },
  });
  const revoke = useMutation({
    mutationFn: (id: string) => {
      if (revokeKey.current?.id !== id)
        revokeKey.current = { id, key: createIdempotencyKey() };
      return teamApi.revokeInvitation(id, revokeKey.current.key);
    },
    onSuccess: (value) => {
      if (link?.id === value.public_id) {
        setLink(null);
        previous.current = null;
      }
      revokeKey.current = null;
      void cache.invalidateQueries({ queryKey: ["team-invitations"] });
    },
  });
  async function copy() {
    try {
      if (!link || !navigator.clipboard?.writeText)
        throw new Error("Copiez le lien affiché dans le champ.");
      await navigator.clipboard.writeText(link.url);
      setNotice("Lien copié. Transmettez-le à la personne invitée.");
    } catch {
      setNotice(
        "La copie automatique n’a pas abouti. Sélectionnez puis copiez le lien affiché.",
      );
    }
  }
  function submit(event: FormEvent) {
    event.preventDefault();
    setNotice(null);
    create.mutate();
  }
  const busy = create.isPending || revoke.isPending;
  return (
    <div className="space-y-5 border-t pt-8">
      <h3 className="flex items-center gap-2 text-lg font-semibold">
        <UserPlus className="size-5 text-primary" />
        Inviter un collègue
      </h3>
      <p className="max-w-3xl text-sm text-muted-foreground">
        Créez un lien à usage unique et transmettez-le vous-même. La personne se
        connectera avant de rejoindre votre entreprise.
      </p>
      {link ? (
        <div className="space-y-3 rounded-lg border border-primary/20 bg-primary/5 p-5">
          <label
            className="block text-sm font-medium"
            htmlFor="team-invitation-link"
          >
            Lien de cette invitation
          </label>
          <Input
            id="team-invitation-link"
            value={link.url}
            readOnly
            onFocus={(event) => event.target.select()}
          />
          <div className="flex flex-wrap gap-3">
            <Button variant="outline" onClick={() => void copy()}>
              <Copy />
              Copier le lien
            </Button>
            <Button
              variant="ghost"
              disabled={busy}
              onClick={() => {
                previous.current = null;
                setLink(null);
                setNotice(null);
                create.reset();
              }}
            >
              Créer une autre invitation
            </Button>
          </div>
          <p className="text-xs text-muted-foreground">
            Ce lien n’est conservé que dans cet onglet. Après fermeture ou
            actualisation, révoquez l’invitation puis créez-en une nouvelle si
            vous ne l’avez pas copié. Aucun e-mail d’invitation n’a été envoyé
            par AI Center.
          </p>
        </div>
      ) : (
        <form onSubmit={submit} className="space-y-4 rounded-lg border p-5">
          <div className="grid gap-4 sm:grid-cols-3">
            <div>
              <label className="mb-2 block text-sm" htmlFor="team-invite-label">
                Repère pour votre suivi
              </label>
              <Input
                id="team-invite-label"
                value={label}
                maxLength={120}
                placeholder="Prénom ou équipe"
                disabled={busy}
                onChange={(event) => setLabel(event.target.value)}
              />
            </div>
            <div>
              <label className="mb-2 block text-sm" htmlFor="team-invite-role">
                Rôle accordé
              </label>
              <select
                id="team-invite-role"
                value={role}
                disabled={busy}
                onChange={(event) =>
                  setRole(event.target.value as InvitationRole)
                }
                className="min-h-10 w-full rounded-md border bg-white px-3 text-sm"
              >
                <option value="editor">Collaborateur</option>
                <option value="viewer">Lecteur</option>
              </select>
            </div>
            <div>
              <label
                className="mb-2 block text-sm"
                htmlFor="team-invite-expiry"
              >
                Durée de validité
              </label>
              <select
                id="team-invite-expiry"
                value={days}
                disabled={busy}
                onChange={(event) => setDays(Number(event.target.value))}
                className="min-h-10 w-full rounded-md border bg-white px-3 text-sm"
              >
                {[1, 3, 7].map((day) => (
                  <option key={day} value={day}>
                    {day} jour{day > 1 ? "s" : ""}
                  </option>
                ))}
              </select>
            </div>
          </div>
          <Button type="submit" disabled={busy}>
            {create.isPending ? "Création…" : "Créer le lien d’invitation"}
          </Button>
        </form>
      )}
      {notice ? (
        <p role="status" className="text-sm text-muted-foreground">
          {notice}
        </p>
      ) : null}
      {create.error || revoke.error ? (
        <ErrorState
          title="L’action n’a pas pu être confirmée"
          error={(create.error ?? revoke.error)!}
        />
      ) : null}
      {list.isPending ? (
        <LoadingState label="Chargement des invitations…" />
      ) : list.isError ? (
        <ErrorState error={list.error} retry={() => void list.refetch()} />
      ) : list.data.length ? (
        <ul className="divide-y rounded-lg border">
          {list.data.map((invitation) => (
            <InvitationRow
              key={invitation.public_id}
              invitation={invitation}
              busy={busy}
              revoke={() => revoke.mutate(invitation.public_id)}
            />
          ))}
        </ul>
      ) : (
        <p className="text-sm text-muted-foreground">
          Aucune invitation créée.
        </p>
      )}
    </div>
  );
}
function InvitationRow({
  invitation,
  busy,
  revoke,
}: {
  invitation: TeamInvitation;
  busy: boolean;
  revoke: () => void;
}) {
  return (
    <li className="flex flex-wrap items-center justify-between gap-4 p-5">
      <div>
        <p className="text-sm font-medium">
          {invitation.label || "Invitation sans repère"}
        </p>
        <p className="mt-1 text-xs text-muted-foreground">
          {roles[invitation.role]} · {invitationStates[invitation.status]} ·
          expire {formatDate(invitation.expires_at, true)}
        </p>
      </div>
      {invitation.status === "pending" ? (
        <Button variant="outline" size="sm" disabled={busy} onClick={revoke}>
          Révoquer l’invitation
        </Button>
      ) : null}
    </li>
  );
}
