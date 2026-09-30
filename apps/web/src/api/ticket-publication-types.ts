import type { Publication, WorkToolProvider } from "./work-tool-types";
export interface PreviewTickets {
  version_id: string;
  connection_id: string;
  expected_provider: WorkToolProvider;
  expected_target_id: string;
  ticket_indexes: number[];
}
export interface PublishTickets extends PreviewTickets {
  preview_fingerprint: string;
  prior_publications_fingerprint: string;
  confirm_additional_issues: boolean;
}
export interface TicketCoverageItem {
  source_ticket_index: number;
  title: string;
  existing_publication: Publication | null;
}
export interface TicketCoverage {
  source_version_number: number;
  artifact_id: string;
  version_id: string;
  provider: WorkToolProvider;
  target_id: string;
  items: TicketCoverageItem[];
}
export interface TicketPreview extends Omit<TicketCoverage, "items"> {
  source_version_number: number;
  content_hash: string;
  connection_id: string;
  connection_revision: number;
  ticket_indexes: number[];
  requested_count: number;
  new_count: number;
  existing_count: number;
  items: (TicketCoverageItem & { business_body_markdown: string })[];
  prior_publications: Publication[];
  prior_publications_fingerprint: string;
  requires_additional_confirmation: boolean;
  preview_fingerprint: string;
  capacity: {
    available_pending: number;
    available_hourly: number;
    max_pending: number;
    max_per_hour: number;
  };
}
export interface TicketPublicationResult extends Omit<TicketCoverage, "items"> {
  source_version_number: number;
  publications: Publication[];
  created_count: number;
  existing_count: number;
}
export interface TicketPublicationReceipt {
  status: string;
  can_retry: boolean;
  result: TicketPublicationResult | null;
}
