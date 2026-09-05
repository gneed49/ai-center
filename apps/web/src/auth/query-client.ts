import { QueryClient } from "@tanstack/react-query";

import { ApiError } from "@/api/client";
import { RequestContextChangedError } from "@/api/request-context";

export function createQueryClient() {
  return new QueryClient({
    defaultOptions: {
      queries: {
        staleTime: 15_000,
        retry: (failureCount, error) =>
          error instanceof RequestContextChangedError ||
          (error instanceof ApiError &&
            error.status >= 400 &&
            error.status < 500)
            ? false
            : failureCount < 1,
        refetchOnWindowFocus: false,
      },
    },
  });
}
