import type { CompanyMember } from "./company-types";
export interface TeamMember extends CompanyMember {
  display_name: string;
}
export type InvitationRole = "editor" | "viewer";
export type InvitationStatus = "pending" | "accepted" | "revoked" | "expired";
export interface TeamInvitation {
  public_id: string;
  role: InvitationRole;
  label: string;
  status: InvitationStatus;
  expires_at: string;
  created_at: string;
  accepted_at: string | null;
}
export interface CreateTeamInvitation {
  public_id: string;
  token_hash: string;
  role: InvitationRole;
  label: string;
  expires_in_days: number;
}
export interface InvitationPreview {
  public_id: string;
  workspace_public_id: string;
  company_name: string;
  role: InvitationRole;
  expires_at: string;
}
