import { toast } from "sonner";

import { RequestContextChangedError } from "@/api/request-context";

/** An abandoned identity's command must not notify its replacement. */
export function notifyRequestError(error: Error) {
  if (!(error instanceof RequestContextChangedError))
    toast.error(error.message);
}
